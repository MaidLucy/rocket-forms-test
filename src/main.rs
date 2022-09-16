#[macro_use] extern crate rocket;

mod stack;
mod db;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(stack::stage())
        .attach(db::stage())
}
