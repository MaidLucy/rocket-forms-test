#[macro_use] extern crate rocket;

use std::borrow::Cow;
use rocket::form::Form;
use rocket::serde::{Serialize, json};
use rocket::State;
use rocket::tokio::sync::Mutex;

#[derive(Debug, FromForm, Serialize)]
#[serde(crate = "rocket::serde")]
struct Message<'r> {
    message: Cow<'r, str>,
    important: Option<bool>,
}

type MessageList = Mutex<Vec<String>>;
type Messages<'r> = &'r State<MessageList>;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[post("/push", data = "<msg>")]
async fn message_save(msg: Form<Message<'_>>, list: Messages<'_>) -> json::Value {
    let mut list = list.lock().await;
    let parsed = msg.into_inner();
    match json::to_string(&parsed).ok() {
        Some(string) => { 
            list.push(string);
            json::json!( { "status": "pushed onto stack" } )
        },
        None => json::json!( { "status": "something went wrong" } ),
    }
}

#[get("/get")]
async fn message_get(list: Messages<'_>) -> json::Value  {
    let mut list = list.lock().await;
    match list.pop() {
        Some(string) => match json::from_str(&string).ok() {
            Some(json) => json,
            None => json::json!( { "status": "oops we didn't get json data from the stack. this should never happen!" } ),
        }
        None => json::json!( { "status": "no more messages" } ),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, message_save, message_get])
        .manage(MessageList::new(vec![]))
}
