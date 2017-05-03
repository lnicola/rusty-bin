#![feature(plugin, custom_derive, custom_attribute, conservative_impl_trait)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate dotenv;
extern crate rocket;
extern crate rocket_contrib;

#[get("/")]
fn hello() -> String {
    "hello world".to_string()
}

fn main() {
    rocket::ignite()
        .mount("/",
               routes![hello])
        .launch();
}
