use chrono::{DateTime, Utc};
use migrations::sqlx;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct IssueRevision {
    pub id: i64,
    pub issue_id: i64,
    pub author_id: i64,
    pub created_at: DateTime<Utc>,
}

impl IssueRevision {
    pub async fn create<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        issue_id: i64,
        author_id: i64,
        executor: E,
    ) -> sqlx::Result<Self> {
        sqlx::query_as!(
            Self,
            r#"INSERT INTO issue_revisions (
                issue_id,
                author_id
            ) VALUES ($1, $2) 
            RETURNING id, issue_id, author_id, created_at"#,
            issue_id,
            author_id
        )
        .fetch_one(executor)
        .await
    }
}

#[derive(Debug, Clone)]
pub struct IssueRevisionChange {
    pub id: i64,
    pub property: String,
    pub issue_revision_id: i64,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

impl IssueRevisionChange {
    pub async fn create<
        'de,
        'e,
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
        S: ToString,
        T: Serialize + Deserialize<'de>,
    >(
        issue_revision_id: i64,
        property: S,
        old_value: Option<T>,
        new_value: Option<T>,
        executor: E,
    ) -> sqlx::Result<Self> {
        let old_value = old_value.map(|v| serde_json::value::to_value(v).unwrap());
        let new_value = new_value.map(|v| serde_json::value::to_value(v).unwrap());
        sqlx::query_as!(
            Self,
            r#"INSERT INTO issue_revision_changes (
                issue_revision_id,
                property,
                old_value,
                new_value
            ) VALUES ($1, $2, $3, $4)
            RETURNING id, issue_revision_id, property, old_value, new_value"#,
            issue_revision_id,
            property.to_string(),
            old_value,
            new_value,
        )
        .fetch_one(executor)
        .await
    }
}
