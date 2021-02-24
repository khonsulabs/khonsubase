use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{DatabaseError, SqlxResultExt};
use migrations::sqlx::{self, Done, FromRow};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub tag_group_id: Option<i32>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            id: 0,
            name,
            tag_group_id: None,
            color: None,
            created_at: Utc::now(),
        }
    }

    pub async fn list_for_issue(issue_id: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(Tag, "SELECT id, name, tag_group_id, color, tags.created_at FROM tags INNER JOIN issue_tags ON issue_tags.tag_id = tags.id WHERE issue_tags.issue_id = $1", issue_id).fetch_all(crate::pool()).await
    }

    pub async fn save<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &mut self,
        executor: E,
    ) -> Result<(), DatabaseError> {
        if self.id == 0 {
            let row = sqlx::query!(
                r#"INSERT INTO tags (
                    name, 
                    tag_group_id, 
                    color
                   ) VALUES ($1, $2, $3) RETURNING id, created_at"#,
                &self.name,
                self.tag_group_id.as_ref(),
                self.color.as_ref(),
            )
            .fetch_one(executor)
            .await
            .map_database_error()?;

            self.id = row.id;
            self.created_at = row.created_at;
        } else {
            let result = sqlx::query!(
                r#"UPDATE tags SET 
                    name = $2,
                    tag_group_id = $3,
                    color = $4
                   WHERE id = $1"#,
                self.id,
                &self.name,
                self.tag_group_id.as_ref(),
                self.color.as_ref()
            )
            .execute(executor)
            .await
            .map_database_error()?;
            if result.rows_affected() == 0 {
                return Err(DatabaseError::RowNotFound);
            }
        }

        Ok(())
    }
}
