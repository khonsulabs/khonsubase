use crate::schema::accounts::User;
use chrono::{DateTime, Utc};
use migrations::sqlx::{self, Done};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, sqlx::Type)]
#[repr(i32)]
pub enum Relationship {
    Blocks = 1,
    Preceeds,
    Causes,
}

#[derive(Debug, Clone)]
pub struct IssueRelationship {
    pub issue_a: i64,
    pub issue_b: i64,
    pub relationship: Option<Relationship>,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl IssueRelationship {
    pub async fn link<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>, S: ToString>(
        issue_a: i64,
        issue_b: i64,
        relationship: Option<Relationship>,
        comment: Option<S>,
        executor: E,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            r#"INSERT INTO issue_relationships (
                issue_a,
                issue_b,
                relationship,
                comment
               ) VALUES ($1, $2, $3, $4)
               ON CONFLICT (issue_a, issue_b) DO UPDATE SET relationship = $3, comment = $4"#,
            issue_a,
            issue_b,
            relationship.map(|r| r as i32),
            comment.map(|s| s.to_string()),
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn unlink<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>, S: ToString>(
        issue_a: i64,
        issue_b: i64,
        executor: E,
    ) -> sqlx::Result<u64> {
        let result = sqlx::query!(
            r#"DELETE FROM issue_relationships WHERE issue_a = $1 AND issue_b = $2"#,
            issue_a,
            issue_b,
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected())
    }
}
