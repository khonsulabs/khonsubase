mod articles;
mod auth;
mod localization;

use std::marker::PhantomData;

use crate::configuration::{Configuration, ConfigurationManager, SiteName};
use localization::UserLanguage;
use rocket::{
    request::{FromRequest, Outcome},
    Request,
};
use rocket_contrib::{
    serve::StaticFiles,
    templates::{tera, Template},
};
use serde::{Deserialize, Serialize};

use self::auth::{SessionData, SessionId};

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
        .mount(
            "/",
            routes![
                auth::signin,
                auth::signin_post,
                auth::signout,
                articles::article_by_slug,
                articles::home
            ],
        )
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

#[derive(Serialize, Deserialize)]
pub struct RequestData {
    pub language: String,
    pub current_path: String,
    pub current_query: Option<String>,
    pub current_path_and_query: String,
    pub session: Option<SessionData>,
}

#[derive(Debug)]
pub struct FullPathAndQuery {
    pub path: String,
    pub query: Option<String>,
}

impl RequestData {
    pub async fn new(
        language: UserLanguage,
        path: FullPathAndQuery,
        session: Option<SessionId>,
    ) -> Self {
        let session = if let Some(session_id) = session {
            session_id.validate().await.ok()
        } else {
            None
        };

        let mut current_path_and_query = path.path.clone();
        if let Some(query) = &path.query {
            current_path_and_query += "?";
            current_path_and_query += query;
        }

        Self {
            language: language.0,
            current_path: path.path,
            current_query: path.query,
            session,
            current_path_and_query,
        }
    }
}

#[rocket::async_trait]
impl<'a, 'r> FromRequest<'a, 'r> for FullPathAndQuery {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        let path = request.uri().path().to_owned();
        let query = request.uri().query().map(|q| q.to_owned());

        Outcome::Success(FullPathAndQuery { path, query })
    }
}
