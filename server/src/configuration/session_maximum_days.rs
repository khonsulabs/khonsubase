use super::Configuration;

pub struct SessionMaximumDays;

impl Configuration for SessionMaximumDays {
    type Type = i64;

    fn default() -> Option<Self::Type> {
        Some(7)
    }

    fn key() -> &'static str {
        "session-maximum-days"
    }
}
