use database::schema::accounts::Account;

pub async fn run() -> anyhow::Result<()> {
    if !Account::any(database::pool()).await? {
        let password = passwords::PasswordGenerator::new()
            .exclude_similar_characters(true)
            .generate_one()
            .unwrap();
        let mut account = Account::new(String::from("admin"), &password)?;
        let mut tx = database::pool().begin().await?;
        account.save(&mut tx).await?;
        tx.commit().await?;

        println!(
            "No accounts found. Generating default user. Username admin, password: {}",
            password
        );
    }

    Ok(())
}
