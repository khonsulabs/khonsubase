use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(
            r#"
                CREATE TABLE issue_relationships (
                    issue_a BIGINT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
                    issue_b BIGINT NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
                    relationship INT NULL,
                    comment TEXT NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                    PRIMARY KEY (issue_a, issue_b)
                )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS issue_relationships")
}
