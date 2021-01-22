use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(r#"ALTER TABLE issues ADD COLUMN started_at TIMESTAMPTZ NULL"#)
        .with_down(r#"ALTER TABLE issues DROP COLUMN IF EXISTS started_at"#)
}
