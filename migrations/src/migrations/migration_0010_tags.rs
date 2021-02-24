use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(
            r#"CREATE TABLE tag_groups (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            single_select BOOL NOT NULL,
            color TEXT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )"#,
        )
        .with_down(r#"DROP TABLE IF EXISTS tag_groups"#)
        .with_up(
            r#"CREATE TABLE tags (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            tag_group_id INT NULL REFERENCES tag_groups(id) ON DELETE CASCADE,
            color TEXT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )"#,
        )
        .with_down(r#"DROP TABLE IF EXISTS tags"#)
        .with_up(
            r#"CREATE TABLE issue_tags (
            issue_id BIGINT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
            tag_id INT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            PRIMARY KEY (issue_id, tag_id)
        )"#,
        )
        .with_down(r#"DROP TABLE IF EXISTS issue_tags"#)
}
