use crate::sqlx::{self, Done};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::Account;

#[derive(Debug)]
pub struct Session {
    pub id: Uuid,
    pub account_id: i64,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Session {
    pub async fn new<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        account: &Account,
        expire_at: Option<DateTime<Utc>>,
        executor: E,
    ) -> Result<Session, sqlx::Error> {
        let new_id = Uuid::new_v4();
        sqlx::query_as!(Session, "INSERT INTO sessions (id, account_id, expires_at) VALUES ($1, $2, $3) RETURNING id, account_id, created_at, expires_at, last_accessed_at", 
            new_id,
            account.id,
            expire_at
        ).fetch_one(executor).await
    }

    pub async fn invalidate_all_except_for<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        account_id: i64,
        session_id: Uuid,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM sessions WHERE account_id = $1 AND id <> $2",
            account_id,
            session_id
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn cleanup<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        executor: E,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM sessions WHERE expires_at IS NOT NULL AND expires_at < NOW()"
        )
        .execute(executor)
        .await?;
        Ok(result.rows_affected())
    }
}
