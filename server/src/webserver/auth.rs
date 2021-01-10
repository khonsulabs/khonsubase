use crate::configuration::{ConfigurationManager, SessionMaximumDays};

use super::{localization::UserLanguage, FullPathAndQuery, RequestData};
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
    request: RequestData,
    username: Option<String>,
    error_message: Option<String>,
    redirect_target: Option<String>,
}

#[get("/signin?<origin>")]
pub async fn signin(
    language: UserLanguage,
    origin: Option<String>,
    session_id: Option<SessionId>,
    path: FullPathAndQuery,
) -> Result<Template, Redirect> {
    if let Some(session_id) = &session_id {
        if session_id.validate().await.is_ok() {
            return Err(Redirect::temporary("/"));
        }
    }

    Ok(Template::render(
        "signin",
        SignInContext {
            request: RequestData::new(language, path, session_id).await,
            username: None,
            redirect_target: origin,
            error_message: None,
        },
    ))
}

#[derive(FromForm, Debug)]
pub struct SignInForm {
    username: String,
    password: String,
    rememberme: bool,
    redirecttarget: Option<String>,
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
            Ok(Redirect::to(
                user.redirecttarget
                    .clone()
                    .unwrap_or_else(|| String::from("/")),
            ))
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
    session_id: Option<SessionId>,
    path: FullPathAndQuery,
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
            request: RequestData::new(language, path, session_id).await,
            redirect_target: user.redirecttarget.clone(),
            username: Some(user.username.clone()),
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

#[get("/signout?<origin>")]
pub async fn signout(cookies: &CookieJar<'_>, origin: Option<String>) -> Redirect {
    cookies.remove(Cookie::named("session_id"));

    if let Some(origin) = origin {
        Redirect::temporary(origin)
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

// #[cfg(test)]
// mod tests {
//     use core::panic;

//     use crate::{
//         test_helpers::{self, TEST_ACCOUNT_PASSWORD, TEST_ACCOUNT_USERNAME},
//         webserver::rocket_server,
//     };
//     use rocket::{
//         http::{ContentType, Status},
//         local::asynchronous::Client,
//     };

//     async fn test_login<T>(user: &str, pass: &str, status: Status, body: T)
//     where
//         T: Into<Option<&'static str>>,
//     {
//         let check_body = body.into();
//         let client = Client::tracked(rocket_server()).await.unwrap();
//         let query = format!("username={}&password={}", user, pass);
//         let response = client
//             .post("/signin")
//             .header(ContentType::Form)
//             .body(&query)
//             .dispatch()
//             .await;

//         let response_status = response.status();
//         let status_matches = response_status == status;
//         let mut body_str = None;

//         let body_matches = if let Some(expected_str) = &check_body {
//             body_str = response.into_string().await;
//             body_str
//                 .as_ref()
//                 .map_or(false, |s| s.contains(expected_str))
//         } else {
//             false
//         };

//         if !status_matches || !body_matches {
//             panic!(
//                 "Unexpected result testing login. Expected {:?}:{:?} but got {:?}:{:?}",
//                 status, check_body, response_status, body_str
//             );
//         }
//     }

//     #[rocket::async_test]
//     async fn test_good_login() -> anyhow::Result<()> {
//         test_helpers::initialize().await;
//         let pool = database::pool();
//         let mut tx = pool.begin().await?;
//         test_helpers::setup_test_account(&mut tx).await?;

//         test_login(
//             TEST_ACCOUNT_USERNAME,
//             TEST_ACCOUNT_PASSWORD,
//             Status::SeeOther,
//             None,
//         )
//         .await;

//         // let form = NewIssueForm {
//         //     summary: String::from("Test Issue"),
//         //     description: None
//         // };
//         // super::create_issue(Form::try_from(form.clone()), UserLanguage(String::from("en-US")), FullPathAndQuery, session)

//         Ok(())
//     }
// }
