mod articles;
mod auth;
mod localization;

use std::marker::PhantomData;

use crate::configuration::{Configuration, ConfigurationManager, SiteName};
use rocket_contrib::{
    serve::StaticFiles,
    templates::{tera, Template},
};

pub async fn main() -> Result<(), rocket::error::Error> {
    rocket::ignite()
        .attach(Template::custom(|engines| {
            engines
                .tera
                .register_function("localize", localization::Localize);
            engines
                .tera
                .register_filter("language_code", localization::LanguageCode);
            engines
                .tera
                .register_function("site_name", TeraConfiguration::<SiteName>::default());
        }))
        .mount("/", routes![auth::signin, auth::signin_post])
        .mount("/", routes![articles::article_by_slug, articles::home])
        .mount("/static", StaticFiles::from("static"))
        .launch()
        .await
}

pub struct TeraConfiguration<T> {
    _phantom: PhantomData<T>,
}

impl<T> Default for TeraConfiguration<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<T> tera::Function for TeraConfiguration<T>
where
    T: Configuration + Send + Sync,
    T::Type: ToString,
{
    fn call(
        &self,
        _args: &std::collections::HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        let manager = ConfigurationManager::shared();
        let value = manager
            .get::<T>()
            .ok_or_else(|| tera::Error::msg("no value found"))?;
        Ok(tera::Value::String(value.to_string()))
    }
}
