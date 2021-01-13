use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(
            r#"
                CREATE TABLE projects (
                    id BIGSERIAL PRIMARY KEY,
                    slug TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    description TEXT NULL,
                    owner_id BIGINT NOT NULL REFERENCES accounts(id),
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
                )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS projects")
        .with_up(
            r#"
                ALTER TABLE issues ADD COLUMN project_id BIGINT NULL REFERENCES projects(id) ON DELETE CASCADE
        "#,
        )
        .with_down("ALTER TABLE issues DROP COLUMN IF EXISTS project_id")
        // Add the administrator flag with default true so that existing users are admins
        .with_up(
            r#"
                ALTER TABLE accounts ADD COLUMN administrator BOOLEAN NOT NULL DEFAULT true
        "#,
        )
        // Drop the default clause so that future users won't be admins
        .with_up(
            r#"
                ALTER TABLE accounts ALTER COLUMN administrator DROP DEFAULT
        "#,
        )
        .with_down("ALTER TABLE accounts DROP COLUMN IF EXISTS administrator")
}
