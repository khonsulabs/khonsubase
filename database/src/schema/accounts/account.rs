use crate::sqlx;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Transaction;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Account {
    pub fn new<S: ToString>(username: S, password: &str) -> anyhow::Result<Account> {
        let mut account = Self {
            id: 0,
            username: username.to_string(),
            password_hash: Default::default(),
            display_name: None,
            created_at: Utc::now(),
        };
        account.set_password_hash(&password)?;
        Ok(account)
    }

    pub async fn any<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        executor: E,
    ) -> sqlx::Result<bool> {
        match sqlx::query!("SELECT id FROM accounts LIMIT 1")
            .fetch_one(executor)
            .await
        {
            Ok(_) => Ok(true),
            Err(sqlx::Error::RowNotFound) => Ok(false),
            Err(err) => Err(err),
        }
    }

    pub async fn find_by_username<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        username: &str,
        executor: E,
    ) -> sqlx::Result<Account> {
        sqlx::query_as!(Account, "SELECT id, username, password_hash, display_name, created_at FROM accounts WHERE username = $1", username).fetch_one(executor).await
    }

    pub async fn find_by_session_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        session_id: Uuid,
        executor: E,
    ) -> sqlx::Result<Account> {
        sqlx::query_as!(Account, "SELECT id, username, password_hash, display_name, created_at FROM accounts WHERE id = validate_session($1)", session_id)
            .fetch_one(executor)
            .await
    }

    pub fn set_password_hash(&mut self, new_password: &str) -> anyhow::Result<()> {
        self.password_hash = bcrypt::hash(new_password.as_bytes(), bcrypt::DEFAULT_COST)?;

        Ok(())
    }

    pub fn verify_password(&self, check_password: &str) -> anyhow::Result<bool> {
        Ok(bcrypt::verify(check_password, &self.password_hash)?)
    }

    pub async fn save(
        &mut self,
        executor: &mut Transaction<'_, sqlx::Postgres>,
    ) -> sqlx::Result<()> {
        if self.id == 0 {
            let row = sqlx::query!(
            "INSERT INTO accounts (username, password_hash, display_name) VALUES ($1, $2, $3) RETURNING id, created_at",
            &self.username,
            &self.password_hash,
            self.display_name.as_ref(),
        )
        .fetch_one(executor)
        .await?;
            self.id = row.id;
            self.created_at = row.created_at;
        } else {
            sqlx::query!(
                "UPDATE accounts SET username = $2, password_hash = $3, display_name = $4 WHERE id = $1", 
                self.id,
                &self.username,
                &self.password_hash,
                self.display_name.as_ref(),
            ).execute(executor).await?;
        }

        Ok(())
    }
}
