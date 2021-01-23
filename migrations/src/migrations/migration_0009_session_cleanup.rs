use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(r#"CREATE INDEX sessions_expires_at ON sessions(expires_at)"#)
        .with_down(r#"DROP INDEX IF EXISTS sessions_expires_at"#)
}
