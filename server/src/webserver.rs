mod articles;
mod auth;
mod issues;
mod localization;

use self::auth::{SessionData, SessionId};

use crate::configuration::{Configuration, ConfigurationManager, SiteName};
use comrak::ComrakOptions;
use localization::UserLanguage;
use rocket::{
    request::{FromRequest, Outcome},
    Request,
};
use rocket_contrib::templates::tera::Value;
use rocket_contrib::{
    serve::StaticFiles,
    templates::{tera, Template},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{env, marker::PhantomData, path::PathBuf};

fn rocket_server() -> rocket::Rocket {
    let root_path = if let Ok(value) = env::var("CARGO_MANIFEST_DIR") {
        let path = PathBuf::from(value);
        path.parent().unwrap().to_path_buf()
    } else {
        std::env::current_dir().unwrap()
    };

    env::set_var(
        "ROCKET_TEMPLATE_DIR",
        dbg!(root_path.join("templates").to_str().unwrap()),
    );

    rocket::ignite()
        .attach(Template::custom(|engines| {
            engines
                .tera
                .register_filter("render_markdown", MarkdownFilter);
            engines
                .tera
                .register_filter("language_code", localization::LanguageCode);
            engines
                .tera
                .register_function("localize", localization::Localize);
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
                articles::home,
                issues::new_issue,
                issues::save_issue,
                issues::edit_issue,
                issues::view_issue,
                issues::list_issues,
            ],
        )
        .mount("/static", StaticFiles::from(root_path.join("static")))
}

pub async fn main() -> Result<(), rocket::error::Error> {
    rocket_server().launch().await
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

#[derive(Debug, Serialize, Deserialize)]
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

    pub fn logged_in(&self) -> bool {
        self.session.is_some()
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

struct MarkdownFilter;

impl tera::Filter for MarkdownFilter {
    fn filter(&self, markdown_source: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
        let markdown = markdown_source.as_str().ok_or_else(|| {
            tera::Error::msg("Value passed to markdown filter needs to be a string")
        })?;
        Ok(Value::String(comrak::markdown_to_html(
            markdown,
            &ComrakOptions::default(),
        )))
    }

    fn is_safe(&self) -> bool {
        true
    }
}
