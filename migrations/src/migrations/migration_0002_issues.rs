use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(
            r#"
                CREATE TABLE issues (
                    id BIGSERIAL PRIMARY KEY,
                    author_id BIGINT NOT NULL REFERENCES accounts(id),
                    summary TEXT NOT NULL,
                    description TEXT NULL,
                    parent_id BIGINT NULL REFERENCES issues(id) ON DELETE CASCADE,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                    completed_at TIMESTAMPTZ NULL
                )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS issues")
        .with_up(
            r#"
                CREATE TABLE issue_revisions (
                    id BIGSERIAL PRIMARY KEY,
                    issue_id BIGINT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
                    author_id BIGINT NOT NULL REFERENCES accounts(id) ON DELETE SET NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
                )
        "#,
        )
        .with_up("ALTER TABLE issues ADD COLUMN current_revision_id BIGINT NULL REFERENCES issue_revisions(id)")
        .with_down("DROP TABLE IF EXISTS issue_revisions")
        .with_down("ALTER TABLE IF EXISTS issues DROP COLUMN current_revision_id")
        .with_up(
            r#"
                CREATE TABLE issue_revision_changes (
                    id BIGSERIAL PRIMARY KEY,
                    issue_revision_id BIGINT NOT NULL REFERENCES issue_revisions(id) ON DELETE CASCADE,
                    property TEXT NOT NULL,
                    old_value JSONB NULL,
                    new_value JSONB NULL
                )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS issue_revision_changes")
        .with_up("ALTER TABLE sessions ADD CONSTRAINT sessions_account_id_cascading_fkey FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE")
        .with_up("ALTER TABLE sessions DROP CONSTRAINT IF EXISTS sessions_account_id_fkey")
        .with_down("ALTER TABLE sessions DROP CONSTRAINT IF EXISTS sessions_account_id_cascading_fkey")
}
