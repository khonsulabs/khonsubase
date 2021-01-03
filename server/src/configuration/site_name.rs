use super::Configuration;

pub struct SiteName;

impl Configuration for SiteName {
    type Type = String;

    fn default() -> Option<Self::Type> {
        Some(String::from("Khonsubase"))
    }

    fn key() -> &'static str {
        "site-name"
    }
}
