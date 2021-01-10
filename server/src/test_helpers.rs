use database::{
    schema::accounts::{Account, Session},
    sqlx,
};
use fluent_templates::once_cell::sync::OnceCell;
use sqlx::{Postgres, Transaction};

pub const TEST_ACCOUNT_USERNAME: &str = "testuser";
pub const TEST_ACCOUNT_PASSWORD: &str = "testpassword";

pub async fn setup_test_account(
    transaction: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<(Account, Session)> {
    let mut account = Account::new(TEST_ACCOUNT_USERNAME, TEST_ACCOUNT_PASSWORD)?;
    account.save(transaction).await?;

    let session = Session::new(&account, None, transaction).await?;

    Ok((account, session))
}

static INITIALIZED: OnceCell<()> = OnceCell::new();

pub async fn initialize() {
    // Only call the async initialization code if the once cell hasn't been initialized
    if INITIALIZED.set(()).is_ok() {
        let _ = dotenv::dotenv();
        database::initialize().await;
    }
}
