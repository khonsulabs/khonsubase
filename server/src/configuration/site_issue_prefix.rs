use super::Configuration;

pub struct SiteIssuePrefix;

impl Configuration for SiteIssuePrefix {
    type Type = String;

    fn default() -> Option<Self::Type> {
        Some(String::from("KB-"))
    }

    fn key() -> &'static str {
        "site-issue-prefix"
    }
}
