use std::collections::HashMap;
use std::iter::FromIterator;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use migrations::sqlx;

use crate::schema::accounts::User;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueRevisionView {
    pub id: i64,
    pub issue_id: i64,
    pub author: User,
    pub created_at: DateTime<Utc>,
    pub changes: HashMap<String, IssueRevisionViewChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueRevisionViewChange {
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

impl IssueRevisionView {
    pub async fn list_for(issue_id: i64) -> sqlx::Result<Vec<Self>> {
        let mut revisions: Vec<Self> = Vec::new();

        for row in         sqlx::query!(
            r#"SELECT 
                issue_revisions.id as id, 
                accounts.id as author_id, 
                accounts.display_name as author_display_name, 
                accounts.username as author_username, 
                issue_revisions.created_at,
                issue_revision_changes.property as "property?",
                issue_revision_changes.old_value,
                issue_revision_changes.new_value
               FROM issue_revisions
               LEFT OUTER JOIN issue_revision_changes ON issue_revision_changes.issue_revision_id = issue_revisions.id
               INNER JOIN accounts ON accounts.id = issue_revisions.author_id
               WHERE issue_revisions.issue_id = $1
               ORDER BY issue_revisions.created_at, issue_revision_changes.property"#,
            issue_id
        )
        .fetch_all(crate::pool())
        .await? {
            if revisions.is_empty() || revisions.last().unwrap().id != row.id {
                let changes = if let Some(property) = row.property {
                    maplit::hashmap!{
                        property => IssueRevisionViewChange {
                            old_value: row.old_value,
                            new_value: row.new_value,
                        }
                    }
                } else {
                    HashMap::default()
                };
                revisions.push(Self {
                    id: row.id,
                    issue_id,
                    author: User {
                        id: row.author_id,
                        username: row.author_username,
                        display_name: row.author_display_name
                    },
                    created_at: row.created_at,
                    changes
                });
            }
        }

        Ok(revisions)
    }
}
