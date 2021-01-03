use super::Configuration;

pub struct SitePrimaryLocale;

impl Configuration for SitePrimaryLocale {
    type Type = String;

    fn default() -> Option<Self::Type> {
        Some(String::from("en-US"))
    }

    fn key() -> &'static str {
        "site-primary-locale"
    }
}
