use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Transaction;
use uuid::Uuid;

use crate::{sqlx, DatabaseError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub username: String,
    pub administrator: bool,
    pub display_name: Option<String>,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(thiserror::Error, Debug)]
pub enum AccountError {
    #[error("username too short")]
    UsernameTooShort,
    #[error("invalid character '{0}'")]
    UsernameInvalidCharacter(char),
    #[error("username already taken")]
    UsernameConflict,
    #[error("sql error: {0}")]
    Sql(#[from] sqlx::Error),
}

impl Account {
    pub fn new<S: ToString>(
        username: S,
        password: &str,
        administrator: bool,
    ) -> anyhow::Result<Account> {
        let mut account = Self {
            id: 0,
            username: username.to_string(),
            administrator,
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

    pub async fn load<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        id: i64,
        executor: E,
    ) -> sqlx::Result<Account> {
        sqlx::query_as!(Self, "SELECT id, username, administrator, password_hash, display_name, created_at FROM accounts WHERE id = $1", id).fetch_one(executor).await
    }

    pub async fn load_for_update<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        id: i64,
        executor: E,
    ) -> sqlx::Result<Account> {
        sqlx::query_as!(Self, "SELECT id, username, administrator, password_hash, display_name, created_at FROM accounts WHERE id = $1 FOR UPDATE", id).fetch_one(executor).await
    }

    pub async fn find_by_username<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        username: &str,
        executor: E,
    ) -> sqlx::Result<Account> {
        sqlx::query_as!(Self, "SELECT id, username, administrator, password_hash, display_name, created_at FROM accounts WHERE username = $1", username).fetch_one(executor).await
    }

    pub async fn find_by_session_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        session_id: Uuid,
        executor: E,
    ) -> sqlx::Result<Account> {
        sqlx::query_as!(Self, "SELECT id, username, administrator, password_hash, display_name, created_at FROM accounts WHERE id = validate_session($1)", session_id)
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
    ) -> Result<(), AccountError> {
        self.username = Account::clean_username(&self.username)?;
        if self.id == 0 {
            let row = sqlx::query!(
            "INSERT INTO accounts (username, password_hash, display_name, administrator) VALUES ($1, $2, $3, $4) RETURNING id, created_at",
            &self.username,
            &self.password_hash,
            self.display_name.as_ref(),
            self.administrator,
        )
        .fetch_one(executor)
        .await?;
            self.id = row.id;
            self.created_at = row.created_at;
        } else {
            sqlx::query!(
                "UPDATE accounts SET username = $2, password_hash = $3, display_name = $4, administrator = $5 WHERE id = $1", 
                self.id,
                &self.username,
                &self.password_hash,
                self.display_name.as_ref(),
                self.administrator,
            ).execute(executor).await?;
        }

        Ok(())
    }

    pub fn clean_username(username: &str) -> Result<String, AccountError> {
        let username = username
            // Strip leading and trailing whitespace
            .trim()
            .chars()
            // Return an error if any remaining character is not valid
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    Ok(c.to_ascii_lowercase())
                } else {
                    Err(AccountError::UsernameInvalidCharacter(c))
                }
            })
            .collect::<Result<String, AccountError>>()?;

        if username.len() < 3 {
            return Err(AccountError::UsernameTooShort);
        }

        Ok(username)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub display_name: Option<String>,
}

impl User {
    pub async fn load<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        id: i64,
        executor: E,
    ) -> sqlx::Result<User> {
        sqlx::query_as!(
            User,
            "SELECT id, username, display_name FROM accounts WHERE id = $1",
            id
        )
        .fetch_one(executor)
        .await
    }

    pub async fn save(
        &mut self,
        executor: &mut Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), AccountError> {
        self.username = Account::clean_username(&self.username)?;
        sqlx::query!(
            "UPDATE accounts SET username = $2, display_name = $3 WHERE id = $1",
            self.id,
            &self.username,
            self.display_name.as_ref(),
        )
        .execute(executor)
        .await
        .map_err(|sql_error| match DatabaseError::from(sql_error) {
            DatabaseError::Conflict => AccountError::UsernameConflict,
            DatabaseError::Other(sql) => AccountError::Sql(sql),
            _ => unreachable!(),
        })?;

        Ok(())
    }
}
