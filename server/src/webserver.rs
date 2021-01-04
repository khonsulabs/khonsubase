mod articles;
mod localization;

use crate::configuration::{Configuration, ConfigurationManager, SiteName};
use localization::UserLanguage;
use rocket_contrib::{
    serve::StaticFiles,
    templates::{tera, Template},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct MarkdownContext {
    language: String,
    markdown: String,
    view_only: bool,
}

#[get("/")]
fn home(language: UserLanguage) -> Template {
    markdown(String::from("home"), language).unwrap()
}

#[get("/<slug>")]
fn markdown(slug: String, language: UserLanguage) -> Result<Template, rocket::http::Status> {
    let article =
        articles::find_article(&slug.to_lowercase()).ok_or(rocket::http::Status::NotFound)?;

    Ok(Template::render(
        "markdown",
        MarkdownContext {
            view_only: true,
            language: language.0,
            markdown: article.body,
        },
    ))
}

pub fn main() {
    rocket::ignite()
        .attach(Template::custom(|engines| {
            engines
                .tera
                .register_filter("localize", localization::tera_localize);
            engines
                .tera
                .register_filter("language_code", localization::language_code);
            engines
                .tera
                .register_function("site_name", tera_configuration::<SiteName>());
        }))
        .mount("/", routes![markdown, home])
        .mount("/static", StaticFiles::from("static"))
        .launch();
}

fn tera_error(message: &str) -> tera::Error {
    tera::ErrorKind::Msg(message.to_owned()).into()
}

pub fn tera_configuration<T>() -> tera::GlobalFn
where
    T: Configuration,
    T::Type: ToString,
{
    Box::new(move |_args| -> tera::Result<tera::Value> {
        let manager = ConfigurationManager::shared();
        let value = manager
            .get::<T>()
            .ok_or_else(|| tera_error("no value found"))?;
        Ok(tera::Value::String(value.to_string()))
    })
}
