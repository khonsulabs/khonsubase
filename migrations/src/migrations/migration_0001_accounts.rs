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
                    account_id BIGINT NOT NULL REFERENCES accounts(id),
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
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
        .with_up(
            "CREATE OR REPLACE FUNCTION validate_session(session_id UUID) RETURNS BIGINT AS $$
                DECLARE
                    session_account_id BIGINT;
                BEGIN
                    DELETE FROM sessions WHERE expires_at IS NOT NULL AND expires_at < now() AND id = session_id;
                    UPDATE sessions SET last_accessed_at = now() WHERE id = session_id RETURNING account_id into session_account_id;
                    IF NOT FOUND THEN
                        return NULL;
                    ELSE
                        return session_account_id;
                    END IF;
                END;
        $$ LANGUAGE plpgsql",
        )
        .with_down("DROP FUNCTION IF EXISTS validate_session(UUID)")
        .debug()
}
