use std::{collections::HashMap, env, marker::PhantomData, path::PathBuf};

use comrak::ComrakOptions;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    response::{Redirect, Responder},
    Request,
};
use rocket_contrib::{
    serve::StaticFiles,
    templates::{tera, tera::Value, Template},
};
use serde::{Deserialize, Serialize};

use database::sqlx;
use localization::UserLanguage;

use crate::configuration::{Configuration, ConfigurationManager, SiteName};

use self::auth::{SessionData, SessionId};
use percent_encoding::{utf8_percent_encode, AsciiSet};
use rocket::http::uri::Uri;
use std::convert::TryInto;

mod articles;
mod auth;
mod issues;
mod localization;
mod projects;
mod users;

fn rocket_server() -> rocket::Rocket {
    let root_path = if let Ok(value) = env::var("CARGO_MANIFEST_DIR") {
        let path = PathBuf::from(value);
        path.parent().unwrap().to_path_buf()
    } else {
        std::env::current_dir().unwrap()
    };

    env::set_var(
        "ROCKET_TEMPLATE_DIR",
        root_path.join("templates").to_str().unwrap(),
    );

    rocket::ignite()
        .attach(Template::custom(|engines| {
            engines
                .tera
                .register_filter("render_markdown", MarkdownFilter);
            engines
                .tera
                .register_filter("language_code", localization::LanguageCode);
            engines.tera.register_filter(
                "relationship_summary_key",
                issues::RelationshipSummaryKeyFilter,
            );
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
                auth::change_password,
                auth::change_password_post,
                articles::article_by_slug,
                articles::home,
                issues::new_issue,
                issues::save_issue,
                issues::edit_issue,
                issues::view_issue,
                issues::list_issues,
                issues::link_issue,
                issues::link_issue_post,
                users::view_user,
                users::edit_user,
                users::save_user,
                users::user_avatar,
                projects::new_project,
                projects::view_project,
                projects::view_project_by_slug,
                projects::edit_project,
                projects::save_project,
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

trait ResultExt<T> {
    fn map_sql_to_http(self) -> Result<T, Status>;

    fn map_to_failure(self) -> Result<T, Failure>
    where
        Self: Sized,
    {
        self.map_sql_to_http().map_err(Failure::Status)
    }
}

impl<T> ResultExt<T> for Result<T, sqlx::Error> {
    fn map_sql_to_http(self) -> Result<T, Status> {
        self.map_err(|err| match err {
            sqlx::Error::RowNotFound => Status::NotFound,
            other_error => {
                error!("unexpected sql error: {:?}", other_error);
                Status::InternalServerError
            }
        })
    }
}

#[derive(Responder)]
#[allow(clippy::large_enum_variant)]
pub enum Failure {
    Status(Status),
    Redirect(Redirect),
}

impl<E> From<E> for Failure
where
    E: std::error::Error,
{
    fn from(error: E) -> Self {
        error!("error processing request: {:?}", error);

        Failure::Status(Status::InternalServerError)
    }
}

const QUERY: &AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`');

impl Failure {
    pub fn redirect<U: TryInto<Uri<'static>>>(destination: U) -> Self {
        Self::Redirect(Redirect::to(destination))
    }

    pub fn redirect_to_signin(origin: Option<&str>) -> Self {
        if let Some(origin) = origin {
            let origin = utf8_percent_encode(origin, QUERY);
            Self::Redirect(Redirect::to(format!("/signin?origin={}", origin)))
        } else {
            Self::Redirect(Redirect::to("/signin"))
        }
    }

    pub fn not_found() -> Self {
        Self::Status(Status::NotFound)
    }

    pub fn forbidden() -> Self {
        Self::Status(Status::Forbidden)
    }
}
