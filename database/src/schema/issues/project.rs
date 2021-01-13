use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use migrations::sqlx::{self, Done, FromRow, Transaction};

use crate::{DatabaseError, SqlxResultExt};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(thiserror::Error, Debug)]
pub enum ProjectError {
    #[error("project not found")]
    ProjectNotFound,
    #[error("invalid character in slug '{0}'")]
    SlugInvalidCharacter(char),
    #[error("slug already in use")]
    SlugConflict,
    #[error("sql error: {0}")]
    Sql(#[from] sqlx::Error),
}

impl From<DatabaseError> for ProjectError {
    fn from(error: DatabaseError) -> Self {
        match error {
            DatabaseError::RowNotFound => ProjectError::ProjectNotFound,
            DatabaseError::Conflict => ProjectError::SlugConflict,
            DatabaseError::Other(sql) => ProjectError::Sql(sql),
        }
    }
}

impl Project {
    pub fn new(slug: String, name: String, description: Option<String>, owner_id: i64) -> Self {
        Self {
            slug,
            name,
            description,
            owner_id,
            id: 0,
            created_at: Utc::now(),
        }
    }

    pub async fn load(project_id: i64) -> sqlx::Result<Self> {
        sqlx::query_as!(
            Self,
            "SELECT id, slug, name, description, owner_id, created_at FROM projects WHERE id = $1",
            project_id
        )
        .fetch_one(crate::pool())
        .await
    }

    pub async fn load_for_update(
        project_id: i64,
        transaction: &mut Transaction<'_, sqlx::Postgres>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as!(Self, "SELECT id, slug, name, description, owner_id, created_at FROM projects WHERE id = $1 FOR UPDATE", project_id).fetch_one(transaction).await
    }

    pub async fn list() -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(Self, "SELECT id, slug, name, description, owner_id, created_at FROM projects ORDER BY lower(name)")
            .fetch_all(crate::pool()).await
    }

    pub async fn save<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &mut self,
        executor: E,
    ) -> Result<(), ProjectError> {
        self.slug = Self::cleanup_and_validate_slug(&self.slug)?;

        if self.id == 0 {
            let row = sqlx::query!(
                r#"INSERT INTO projects (
                    slug, 
                    name, 
                    description, 
                    owner_id
                   ) VALUES ($1, $2, $3, $4) RETURNING id, created_at"#,
                &self.slug,
                &self.name,
                self.description.as_ref(),
                self.owner_id,
            )
            .fetch_one(executor)
            .await
            .map_database_error()?;

            self.id = row.id;
            self.created_at = row.created_at;
        } else {
            let result = sqlx::query!(
                r#"UPDATE projects SET 
                    slug = $2,
                    name = $3,
                    description = $4,
                    owner_id = $5
                   WHERE id = $1"#,
                self.id,
                &self.slug,
                &self.name,
                self.description.as_ref(),
                self.owner_id,
            )
            .execute(executor)
            .await
            .map_database_error()?;
            if result.rows_affected() == 0 {
                return Err(ProjectError::ProjectNotFound);
            }
        }

        Ok(())
    }

    pub fn cleanup_and_validate_slug(slug: &str) -> Result<String, ProjectError> {
        let mut cleaned = String::new();
        for c in slug.trim().chars() {
            if !(c.is_ascii_alphanumeric() || c == '-') {
                return Err(ProjectError::SlugInvalidCharacter(c));
            }
            cleaned.push(c.to_ascii_lowercase());
        }

        Ok(cleaned)
    }
}
