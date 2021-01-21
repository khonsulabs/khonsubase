use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(
            r#"
            CREATE OR REPLACE VIEW issue_blocked_statuses AS
            SELECT
                issues.id,
                CASE WHEN COUNT(blocker.id) = 0 THEN FALSE ELSE bool_or(blocker.completed_at IS NULL) END AS blocked
            FROM issues
            LEFT OUTER JOIN issue_relationships ON (issue_relationships.relationship = 1 AND issue_relationships.issue_b = issues.id)
            LEFT OUTER JOIN issues blocker ON blocker.id = issue_relationships.issue_a 
            GROUP BY issues.id;
            "#,
        )
}
