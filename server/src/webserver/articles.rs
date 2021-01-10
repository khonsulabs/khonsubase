use crate::webserver::localization::UserLanguage;
use database::schema::cms::Article;
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

use super::{auth::SessionId, FullPathAndQuery, RequestData};

pub fn find_article(slug: &str) -> Option<Article> {
    if slug == "home" {
        Some(Article::hardcoded(slug, "# Welcome to Khonsubase\n\nThis is a pre-alpha [work-in-progress project](https://github.com/khonsulabs/khonsubase)."))
    } else if slug == "terms-of-service" {
        Some(Article::hardcoded(slug, "# Welcome to Khonsubase\n\nThis is a pre-alpha [work-in-progress project](https://github.com/khonsulabs/khonsubase). There is no Terms of Service at this time."))
    } else if slug == "privacy-policy" {
        Some(Article::hardcoded(slug, "# Privacy Policy\n\nThis is a pre-alpha [work-in-progress project](https://github.com/khonsulabs/khonsubase). There is no Privacy Policy at this time."))
    } else {
        None
    }
}

#[derive(Serialize, Deserialize)]
struct MarkdownContext {
    slug: String,
    request: RequestData,
    markdown: String,
    view_only: bool,
}

#[get("/")]
pub async fn home(
    language: UserLanguage,
    session: Option<SessionId>,
    path: FullPathAndQuery,
) -> Template {
    article_by_slug(String::from("home"), language, session, path)
        .await
        .unwrap()
}

#[get("/<slug>")]
pub async fn article_by_slug(
    slug: String,
    language: UserLanguage,
    session: Option<SessionId>,
    path: FullPathAndQuery,
) -> Result<Template, rocket::http::Status> {
    let article = find_article(&slug.to_lowercase()).ok_or(rocket::http::Status::NotFound)?;
    Ok(render_article(article, language, session, path).await)
}

async fn render_article(
    article: Article,
    language: UserLanguage,
    session: Option<SessionId>,
    path: FullPathAndQuery,
) -> Template {
    Template::render(
        "markdown",
        MarkdownContext {
            slug: article.slug,
            request: RequestData::new(language, path, session).await,
            view_only: true,
            markdown: article.body,
        },
    )
}
