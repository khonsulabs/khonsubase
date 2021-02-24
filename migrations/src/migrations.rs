mod migration_0001_accounts;
mod migration_0002_issues;
mod migration_0003_issue_comments;
mod migration_0004_projects;
mod migration_0005_relations;
mod migration_0006_blocking;
mod migration_0007_blocking_fix;
mod migration_0008_started_at;
mod migration_0009_session_cleanup;
mod migration_0010_tags;

use crate::connection::pool;
use sqlx_simple_migrator::{Migration, MigrationError};

pub fn migrations() -> Vec<Migration> {
    vec![
        migration_0001_accounts::migration(),
        migration_0002_issues::migration(),
        migration_0003_issue_comments::migration(),
        migration_0004_projects::migration(),
        migration_0005_relations::migration(),
        migration_0006_blocking::migration(),
        migration_0007_blocking_fix::migration(),
        migration_0008_started_at::migration(),
        migration_0009_session_cleanup::migration(),
        migration_0010_tags::migration(),
    ]
}

pub async fn run_all() -> Result<(), MigrationError> {
    let pool = pool();

    Migration::run_all(&pool, migrations()).await
}
