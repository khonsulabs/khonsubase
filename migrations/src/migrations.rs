mod migration_0001_accounts;
mod migration_0002_issues;
mod migration_0003_issue_comments;
mod migration_0004_projects;
mod migration_0005_relations;
mod migration_0006_blocking;

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
    ]
}

pub async fn run_all() -> Result<(), MigrationError> {
    let pool = pool();

    Migration::run_all(&pool, migrations()).await
}
