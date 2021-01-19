use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        // Add the blocked flag with a default value
        .with_up(
            r#"
                ALTER TABLE issues ADD COLUMN blocked BOOLEAN NOT NULL DEFAULT false
        "#,
        )
        // Drop the default clause so that issues in the future won't automatically get a value
        .with_up(
            r#"
                ALTER TABLE issues ALTER COLUMN blocked DROP DEFAULT
        "#,
        )
        // Initialize the correct blocked statuses for all issues
        .with_up(
            r#"
            CREATE OR REPLACE VIEW issue_blocked_statuses AS
            SELECT
                issues.id,
                CASE WHEN COUNT(blocker.id) = 0 THEN FALSE ELSE bool_or(blocker.completed_at IS NULL) END AS blocked
            FROM issues
                     LEFT OUTER JOIN issue_relationships ON issue_relationships.issue_b = issues.id
                     LEFT OUTER JOIN issues blocker ON blocker.id = issue_relationships.issue_a
            GROUP BY issues.id;
            "#,
        )
        .with_down("DROP VIEW IF EXISTS issue_blocked_statuses")
        .with_up(
            r#"
            UPDATE issues SET blocked = issue_blocked_statuses.blocked
            FROM issue_blocked_statuses
            WHERE issues.id = issue_blocked_statuses.id
        "#,
        )
        .with_down("ALTER TABLE issues DROP COLUMN IF EXISTS blocked")
}
