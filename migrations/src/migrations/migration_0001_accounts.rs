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
                CREATE TABLE sessions (
                    id UUID PRIMARY KEY,
                    account_id BIGINT NULL REFERENCES accounts(id),
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                    expires_at TIMESTAMPTZ NULL
                )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS sessions")
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
