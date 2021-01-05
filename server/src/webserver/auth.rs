use super::localization::UserLanguage;
use database::{schema::accounts::Account, sqlx};
use rocket::{request::Form, response::Redirect};
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SignInContext {
    language: String,
    error_message: Option<String>,
}

#[get("/signin")]
pub fn signin(language: UserLanguage) -> Template {
    Template::render(
        "signin",
        SignInContext {
            language: language.0,
            error_message: None,
        },
    )
}

#[derive(FromForm, Debug)]
pub struct SignInForm {
    username: String,
    password: String,
    rememberme: bool,
}

#[post("/signin", data = "<user>")]
pub async fn signin_post(
    user: Form<SignInForm>,
    language: UserLanguage,
) -> Result<Template, Redirect> {
    let error_message = match Account::find_by_username(&user.username, database::pool()).await {
        Ok(account) => match account.verify_password(&user.password) {
            Ok(true) => return Err(Redirect::to("/")),
            Ok(false) => "sign-in-error-user-not-found",
            Err(_) => "sign-in-error-internal-error",
        },
        Err(sqlx::Error::RowNotFound) => "sign-in-error-user-not-found",
        Err(_) => "sign-in-error-internal-error",
    };

    Ok(Template::render(
        "signin",
        SignInContext {
            language: language.0,
            error_message: Some(error_message.to_owned()),
        },
    ))
}
