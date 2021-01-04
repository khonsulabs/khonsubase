mod migration_0001_accounts;

use crate::connection::pool;
use sqlx_simple_migrator::{Migration, MigrationError};

pub fn migrations() -> Vec<Migration> {
    vec![migration_0001_accounts::migration()]
}

pub async fn run_all() -> Result<(), MigrationError> {
    let pool = pool();

    Migration::run_all(&pool, migrations()).await
}
