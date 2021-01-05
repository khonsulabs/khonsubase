#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod configuration;
mod setup;
mod webserver;

#[rocket::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().unwrap();

    database::initialize().await;

    database::migrations::run_all()
        .await
        .expect("error executing database migrations");

    setup::run().await.expect("error executing setup");

    webserver::main().await?;

    Ok(())
}
