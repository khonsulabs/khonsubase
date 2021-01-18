#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod configuration;
mod setup;
mod webserver;

mod kbql;
#[cfg(test)]
#[allow(dead_code)]
mod test_helpers;

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

trait Optionable: Sized {
    fn into_option(self) -> Option<Self>;
}

impl Optionable for &str {
    fn into_option(self) -> Option<Self> {
        if self.trim().is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

impl Optionable for String {
    fn into_option(self) -> Option<Self> {
        if self.trim().is_empty() {
            None
        } else {
            Some(self)
        }
    }
}
