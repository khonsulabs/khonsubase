#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod configuration;
mod setup;
mod webserver;

fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().unwrap();

    let mut tokio = tokio::runtime::Runtime::new()?;
    tokio.block_on(async {
        database::initialize().await;

        database::migrations::run_all()
            .await
            .expect("error executing database migrations");

        setup::run().await.expect("error executing setup");
    });

    webserver::main();

    Ok(())
}
