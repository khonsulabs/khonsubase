use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{export::Formatter, Deserialize, Serialize};

use migrations::sqlx::{self, Done};

#[derive(Clone, Copy, Debug, sqlx::Type, Serialize, Deserialize)]
#[repr(i32)]
pub enum Relationship {
    Blocks = 1,
    Preceeds,
    Causes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueRelationship {
    pub issue_id: i64,
    pub issue_summary: String,
    pub issue_completed_at: Option<DateTime<Utc>>,
    pub issue_project_id: Option<i64>,
    pub relationship: Option<ContextualizedRelationship>,
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

    pub async fn unlink<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        issue_a: i64,
        issue_b: i64,
        executor: E,
    ) -> sqlx::Result<u64> {
        let result = sqlx::query!(
            r#"DELETE FROM issue_relationships WHERE (issue_a = $1 AND issue_b = $2) OR (issue_b = $1 AND issue_a = $2)"#,
            issue_a,
            issue_b,
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn list_for<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        issue_id: i64,
        executor: E,
    ) -> sqlx::Result<Vec<Self>> {
        // This query is complicated due to its use of CASE WHEN as a ternary. To only return the
        // relevant data from the database, we check on each issue_ field whether the queried issue
        // is the "a" side. If so, then the related issue being returned is the "b" issue. Otherwise,
        // it is considered an inverse_relationship and the issue_a data is the one we need to return.
        let rows = sqlx::query!(
            r#"SELECT 
                CASE WHEN issue_a.id = $1 THEN issue_b.id ELSE issue_a.id END as "issue_id!",
                CASE WHEN issue_a.id = $1 THEN issue_b.project_id ELSE issue_a.project_id END as issue_project_id,
                CASE WHEN issue_a.id = $1 THEN issue_b.summary ELSE issue_a.summary END as "issue_summary!",
                CASE WHEN issue_a.id = $1 THEN issue_b.completed_at ELSE issue_a.completed_at END as issue_completed_at,
                CASE WHEN issue_a.id = $1 THEN FALSE ELSE TRUE END as "inverse_relationship!",
                relationship as "relationship: Relationship",
                comment,
                issue_relationships.created_at
               FROM issue_relationships
               INNER JOIN issues issue_a ON issue_a.id = issue_relationships.issue_a
               INNER JOIN issues issue_b ON issue_b.id = issue_relationships.issue_b
               WHERE issue_relationships.issue_a = $1 OR issue_relationships.issue_b = $1"#,
            issue_id,
        )
        .fetch_all(executor)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Self {
                relationship: row
                    .relationship
                    .as_ref()
                    .map(|r| ContextualizedRelationship::new(*r, row.inverse_relationship)),
                issue_id: row.issue_id,
                issue_summary: row.issue_summary,
                issue_completed_at: row.issue_completed_at,
                issue_project_id: row.issue_project_id,
                comment: row.comment,
                created_at: row.created_at,
            })
            .collect())
    }

    pub async fn find<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        issue_one: i64,
        issue_two: i64,
        executor: E,
    ) -> sqlx::Result<Self> {
        // This query is the same as from list_for, except the filter at the bottom includes the second ID test
        let row = sqlx::query!(
            r#"SELECT 
                CASE WHEN issue_a.id = $1 THEN issue_b.id ELSE issue_a.id END as "issue_id!",
                CASE WHEN issue_a.id = $1 THEN issue_b.project_id ELSE issue_a.project_id END as issue_project_id,
                CASE WHEN issue_a.id = $1 THEN issue_b.summary ELSE issue_a.summary END as "issue_summary!",
                CASE WHEN issue_a.id = $1 THEN issue_b.completed_at ELSE issue_a.completed_at END as issue_completed_at,
                CASE WHEN issue_a.id = $1 THEN FALSE ELSE TRUE END as "inverse_relationship!",
                relationship as "relationship: Relationship",
                comment,
                issue_relationships.created_at
               FROM issue_relationships
               INNER JOIN issues issue_a ON issue_a.id = issue_relationships.issue_a
               INNER JOIN issues issue_b ON issue_b.id = issue_relationships.issue_b
               WHERE (issue_relationships.issue_a = $1 AND issue_relationships.issue_b = $2) OR (issue_relationships.issue_b = $1 AND issue_relationships.issue_a = $2)"#,
            issue_one,
            issue_two
        )
            .fetch_one(executor)
            .await?;

        Ok(Self {
            relationship: row
                .relationship
                .as_ref()
                .map(|r| ContextualizedRelationship::new(*r, row.inverse_relationship)),
            issue_id: row.issue_id,
            issue_summary: row.issue_summary,
            issue_completed_at: row.issue_completed_at,
            issue_project_id: row.issue_project_id,
            comment: row.comment,
            created_at: row.created_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualizedRelationship {
    pub relationship: Option<Relationship>,
    pub is_inverse: bool,
}

impl ContextualizedRelationship {
    pub fn new(relationship: Relationship, is_inverse: bool) -> Self {
        Self {
            relationship: Some(relationship),
            is_inverse,
        }
    }

    pub fn plain() -> Self {
        Self {
            relationship: None,
            is_inverse: false,
        }
    }
}

impl std::fmt::Display for ContextualizedRelationship {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let string = match (self.relationship, self.is_inverse) {
            (None, false) | (None, true) => "relates",
            (Some(Relationship::Blocks), false) => "blocks",
            (Some(Relationship::Blocks), true) => "blocked",
            (Some(Relationship::Preceeds), false) => "precedes",
            (Some(Relationship::Preceeds), true) => "preceded",
            (Some(Relationship::Causes), false) => "causes",
            (Some(Relationship::Causes), true) => "caused",
        };

        f.write_str(string)
    }
}

impl FromStr for ContextualizedRelationship {
    type Err = RelationshipParseError;

    fn from_str(relationship: &str) -> Result<Self, Self::Err> {
        match relationship {
            "relates" => Ok(ContextualizedRelationship::plain()),
            "blocks" => Ok(ContextualizedRelationship::new(Relationship::Blocks, false)),
            "blocked" => Ok(ContextualizedRelationship::new(Relationship::Blocks, true)),
            "precedes" => Ok(ContextualizedRelationship::new(
                Relationship::Preceeds,
                false,
            )),
            "preceded" => Ok(ContextualizedRelationship::new(
                Relationship::Preceeds,
                true,
            )),
            "causes" => Ok(ContextualizedRelationship::new(Relationship::Causes, false)),
            "caused" => Ok(ContextualizedRelationship::new(Relationship::Causes, true)),
            _ => Err(RelationshipParseError),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("invalid relationship")]
pub struct RelationshipParseError;
