use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(
            r#"
                ALTER TABLE issue_revisions ADD COLUMN comment TEXT NULL
        "#,
        )
        .with_down("ALTER TABLE issue_revisions DROP COLUMN IF EXISTS comment")
}
