use super::localization::UserLanguage;
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SignInContext {
    language: String,
}

#[get("/signin")]
pub fn signin(language: UserLanguage) -> Template {
    Template::render(
        "signin",
        SignInContext {
            language: language.0,
        },
    )
}
