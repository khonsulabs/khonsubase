use super::Configuration;

pub struct SiteDefaultTimezone;

impl Configuration for SiteDefaultTimezone {
    type Type = String;

    fn default() -> Option<Self::Type> {
        Some(String::from("US/Pacific"))
    }

    fn key() -> &'static str {
        "site-default-timezone"
    }
}

impl SiteDefaultTimezone {
    pub fn get_for_chrono() -> chrono_tz::Tz {
        Self::get()
            .map(|name| name.parse().expect("Invalid time zone identifier"))
            .unwrap()
    }
}
