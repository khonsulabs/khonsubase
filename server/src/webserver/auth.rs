use crate::configuration::{ConfigurationManager, SessionMaximumDays};

use super::localization::UserLanguage;
use database::{
    schema::accounts::{Account, Session},
    sqlx::{self, types::chrono::Utc},
};
use rocket::{
    http::{Cookie, CookieJar, SameSite},
    outcome::IntoOutcome,
    request::{Form, FromRequest, Outcome},
    response::Redirect,
    Request,
};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct SignInContext {
    language: String,
    error_message: Option<String>,
}

#[get("/signin")]
pub async fn signin(
    language: UserLanguage,
    session_id: Option<SessionId>,
) -> Result<Template, Redirect> {
    if let Some(session_id) = session_id {
        if session_id.validate().await.is_ok() {
            return Err(Redirect::temporary("/"));
        }
    }

    Ok(Template::render(
        "signin",
        SignInContext {
            language: language.0,
            error_message: None,
        },
    ))
}

#[derive(FromForm, Debug)]
pub struct SignInForm {
    username: String,
    password: String,
    rememberme: bool,
}

async fn verify_account(
    account: &Account,
    user: &Form<SignInForm>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, SignInError> {
    match account.verify_password(&user.password) {
        Ok(true) => {
            let session_maximum_days = ConfigurationManager::shared()
                .get::<SessionMaximumDays>()
                .unwrap();
            let cookie_duration = if user.rememberme {
                Some(time_01::Duration::days(session_maximum_days))
            } else {
                None
            };
            let session = Session::new(
                &account,
                cookie_duration.map(|d| Utc::now() + d),
                database::pool(),
            )
            .await
            .map_err(|_| SignInError::InternalError)?;
            let mut cookie = Cookie::new("session_id", session.id.to_string());
            cookie.set_http_only(true);
            cookie.set_same_site(SameSite::Strict);

            if user.rememberme {
                // While this cookie is "permanent", the session will expire server-side during cleanup operations based on last access time.
                cookie.make_permanent();
            } else {
                // Session cookie
                cookie.set_expires(None);
            }

            cookie.set_path("/");
            cookies.add(cookie);
            Ok(Redirect::to("/"))
        }
        Ok(false) => Err(SignInError::UserNotFound),
        Err(_) => Err(SignInError::InternalError),
    }
}

enum SignInError {
    UserNotFound,
    InternalError,
}

impl ToString for SignInError {
    fn to_string(&self) -> String {
        match self {
            SignInError::UserNotFound => "sign-in-error-user-not-found",
            SignInError::InternalError => "sign-in-error-internal-error",
        }
        .to_string()
    }
}

#[post("/signin", data = "<user>")]
pub async fn signin_post(
    user: Form<SignInForm>,
    language: UserLanguage,
    cookies: &CookieJar<'_>,
) -> Result<Template, Redirect> {
    let error_message = match Account::find_by_username(&user.username, database::pool()).await {
        Ok(account) => match verify_account(&account, &user, cookies).await {
            Ok(redirect) => return Err(redirect),
            Err(message) => message,
        },
        Err(sqlx::Error::RowNotFound) => SignInError::UserNotFound,
        Err(_) => SignInError::InternalError,
    };

    Ok(Template::render(
        "signin",
        SignInContext {
            language: language.0,
            error_message: Some(error_message.to_string()),
        },
    ))
}

#[derive(Debug)]
pub struct SessionId(pub Uuid);

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for SessionId {
    type Error = std::convert::Infallible;

    async fn from_request(
        request: &'a rocket::Request<'r>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        request
            .cookies()
            .get("session_id")
            .and_then(|cookie| cookie.value().parse().ok())
            .map(SessionId)
            .or_forward(())
    }
}

impl SessionId {
    pub async fn validate(&self) -> Result<SessionData, sqlx::Error> {
        let account = Account::find_by_session_id(self.0, database::pool()).await?;

        Ok(SessionData { account })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionData {
    pub account: Account,
}

#[get("/signout")]
pub async fn signout(cookies: &CookieJar<'_>, referer: Option<Referer>) -> Redirect {
    cookies.remove(Cookie::named("session_id"));

    if let Some(referer) = referer {
        Redirect::temporary(referer.0)
    } else {
        Redirect::temporary("/")
    }
}

#[derive(Debug)]
pub struct Referer(pub String);

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for Referer {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        request
            .headers()
            .get_one("Referer")
            .map(|r| Referer(r.to_owned()))
            .or_forward(())
    }
}
