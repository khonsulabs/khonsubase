use database::schema::cms::Article;
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
