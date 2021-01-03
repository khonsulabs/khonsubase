#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

mod localization;

use localization::UserLanguage;
use rocket_contrib::{serve::StaticFiles, templates::Template};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct IndexContext {
    language: String,
}

#[get("/")]
fn hello(language: UserLanguage) -> Template {
    Template::render(
        "index",
        IndexContext {
            language: language.0,
        },
    )
}

fn main() {
    rocket::ignite()
        .attach(Template::custom(|engines| {
            engines
                .tera
                .register_filter("localize", localization::tera_localize);
            engines
                .tera
                .register_filter("language_code", localization::language_code);
        }))
        .mount("/", routes![hello])
        .mount("/static", StaticFiles::from("static"))
        .launch();
}
