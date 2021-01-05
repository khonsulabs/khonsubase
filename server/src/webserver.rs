mod articles;
mod auth;
mod localization;

use crate::configuration::{Configuration, ConfigurationManager, SiteName};
use rocket_contrib::{
    serve::StaticFiles,
    templates::{tera, Template},
};

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
        .mount("/", routes![auth::signin])
        .mount("/", routes![articles::article_by_slug, articles::home])
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
