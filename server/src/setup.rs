mod initial_admin_user;

pub async fn run() -> anyhow::Result<()> {
    initial_admin_user::run().await?;

    Ok(())
}
