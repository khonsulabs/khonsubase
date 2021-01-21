use crate::webserver::localization::UserLanguage;
use database::schema::cms::Article;
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

use super::{auth::SessionId, FullPathAndQuery, RequestData};
use crate::configuration::{Configuration, SiteIssuePrefix};

pub fn find_article(slug: &str) -> Option<Article> {
    if slug == "home" {
        Some(Article::hardcoded(slug, "# Welcome to Khonsubase\n\nThis is a pre-alpha [work-in-progress project](https://github.com/khonsulabs/khonsubase)."))
    } else if slug == "terms-of-service" {
        Some(Article::hardcoded(slug, "# Welcome to Khonsubase\n\nThis is a pre-alpha [work-in-progress project](https://github.com/khonsulabs/khonsubase). There is no Terms of Service at this time."))
    } else if slug == "privacy-policy" {
        Some(Article::hardcoded(slug, "# Privacy Policy\n\nThis is a pre-alpha [work-in-progress project](https://github.com/khonsulabs/khonsubase). There is no Privacy Policy at this time."))
    } else if slug == "signup" {
        Some(Article::hardcoded(slug, "# Sign Up\n\nSigning up is not currently supported. You can track progress towards this being supported [here](https://base.khonsulabs.com/issue/21)."))
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
            markdown: preformat_markdown(&article.body),
        },
    )
}

pub fn preformat_markdown(markdown: &str) -> String {
    let mut formatted = String::with_capacity(markdown.len());
    let issue_prefix = SiteIssuePrefix::get().unwrap();
    let mut parts = markdown.split(&issue_prefix);
    formatted.push_str(parts.next().unwrap());

    for part in parts {
        // Find the first non-ascii-digit
        let mut number_length = 0usize;
        for c in part.chars() {
            if c.is_ascii_digit() {
                number_length += 1;
            } else {
                break;
            }
        }

        if number_length > 0 {
            let number = &part[0..number_length];
            let remaining_part = &part[number_length..];
            formatted.push('[');
            formatted.push_str(&issue_prefix);
            formatted.push_str(number);
            formatted.push_str("](/issue/");
            formatted.push_str(number);
            formatted.push(')');
            formatted.push_str(remaining_part);
        } else {
            formatted.push_str(&issue_prefix);
            formatted.push_str(part);
        }
    }

    formatted
}
