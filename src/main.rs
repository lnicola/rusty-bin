#![feature(plugin, custom_derive, custom_attribute, conservative_impl_trait)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate diesel;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate dotenv;
extern crate rocket;
extern crate rocket_contrib;

mod db;

use std::env;

use db::Conn;

#[get("/")]
fn hello() -> String {
    "hello world".to_string()
}

fn main() {
    dotenv::dotenv().ok();

    rocket::ignite()
        .manage(db::init_pool(&env::var("DATABASE_URL").expect("DATABASE_URL must be set")))
        .mount("/", routes![hello])
        .launch();
}
