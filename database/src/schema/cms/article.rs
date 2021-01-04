pub struct Article {
    pub id: i64,
    pub slug: String,
    pub body: String,
}

impl Article {
    pub fn hardcoded(slug: &str, body: &str) -> Self {
        Self {
            id: 0,
            slug: slug.to_owned(),
            body: body.to_owned(),
        }
    }
}
