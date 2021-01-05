use crate::webserver::localization::UserLanguage;
use database::schema::cms::Article;
use rocket_contrib::templates::Template;
use serde::{Deserialize, Serialize};

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
    language: String,
    markdown: String,
    view_only: bool,
}

#[get("/")]
pub fn home(language: UserLanguage) -> Template {
    article_by_slug(String::from("home"), language).unwrap()
}

#[get("/<slug>")]
pub fn article_by_slug(
    slug: String,
    language: UserLanguage,
) -> Result<Template, rocket::http::Status> {
    let article = find_article(&slug.to_lowercase()).ok_or(rocket::http::Status::NotFound)?;
    Ok(render_article(article, language))
}

fn render_article(article: Article, language: UserLanguage) -> Template {
    Template::render(
        "markdown",
        MarkdownContext {
            view_only: true,
            language: language.0,
            markdown: article.body,
        },
    )
}
