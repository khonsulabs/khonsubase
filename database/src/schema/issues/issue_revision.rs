use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct IssueRevision {
    pub id: i64,
    pub issue_id: i64,
    pub author_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct IssueRevisionChange {
    pub id: i64,
    pub issue_revision_id: i64,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

impl IssueRevisionChange {
    pub fn new<'de, T: Serialize + Deserialize<'de>>(
        issue_revision_id: i64,
        old_value: Option<T>,
        new_value: Option<T>,
    ) -> Self {
        Self {
            id: 0,
            issue_revision_id,
            old_value: old_value.map(|v| serde_json::value::to_value(v).unwrap()),
            new_value: new_value.map(|v| serde_json::value::to_value(v).unwrap()),
        }
    }
}
