use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(
            r#"
                CREATE TABLE accounts (
                    id BIGSERIAL PRIMARY KEY,
                    username TEXT NOT NULL UNIQUE,
                    password_hash TEXT NOT NULL,
                    display_name TEXT NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
                )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS accounts")
        .with_up(
            r#"
                CREATE TABLE account_emails (
                    account_id BIGSERIAL,
                    email TEXT,
                    hashed_email TEXT NOT NULL,
                    is_primary BOOLEAN NOT NULL,
                    CONSTRAINT account_emails_pkey PRIMARY KEY (account_id, hashed_email),
                    CONSTRAINT account_emails_unique UNIQUE (account_id, hashed_email)
                )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS account_emails")
        .with_up(
            r#"
                CREATE TABLE installations (
                    id UUID PRIMARY KEY,
                    account_id BIGINT NULL REFERENCES accounts(id),
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                    last_connected_at TIMESTAMPTZ NOT NULL DEFAULT now()
                )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS installations")
        .with_up(
            r#"
                CREATE TABLE account_agreements (
                    account_id BIGINT NULL REFERENCES accounts(id),
                    agreement TEXT NOT NULL,
                    version_agreed_to TEXT NOT NULL,
                    agreed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                    CONSTRAINT account_agreements_pkey PRIMARY KEY (account_id, agreement)
                )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS account_agreements")
        .debug()
}
