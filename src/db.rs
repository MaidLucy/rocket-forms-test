use std::borrow::Cow;
use std::collections::HashMap;
use rocket::{Rocket, Build, futures};
use rocket::fairing::{self, AdHoc};
use rocket::form::Form;
use rocket::response::status::Created;
use rocket::serde::{Serialize, Deserialize, json};

use rocket_db_pools::{sqlx, Database, Connection};

// use futures::{stream::TryStreamExt, future::TryFutureExt};
use rocket_dyn_templates::{Template, context};

#[derive(Database)]
#[database("sqlx")]
struct Db(sqlx::SqlitePool);

type Result<T, E = rocket::response::Debug<sqlx::Error>> = std::result::Result<T, E>;

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message<'r> {
    message: Cow<'r, str>,
    important: Option<bool>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct DbMessage {
    id: i64,
    message: String,
    important: Option<bool>,
}

#[get("/submit")]
fn message_submit_form() -> Template {
    let mut context = HashMap::new();
    context.insert("title", "Submit message");
    Template::render("message_submit", context)
}

#[post("/submit", data = "<msg>")]
async fn message_submit(mut db: Connection<Db>, msg: Form<Message<'_>>) -> Result<Created<&str>> {
    sqlx::query!("INSERT INTO messages (message, important) VALUES (?, ?)", msg.message, msg.important)
        .execute(&mut *db).await?;
    Ok(Created::new("/messages").body("success"))
}

#[get("/messages/json")]
async fn list_messages_json<'r>(mut db: Connection<Db>) -> Result<json::Json<Vec<DbMessage>>> {
    let query = sqlx::query_as!(DbMessage, "SELECT * FROM messages")
        .fetch_all(&mut *db).await?;
    Ok(json::Json(query))
}

#[get("/messages")]
async fn list_messages(mut db: Connection<Db>) -> Template {
    let query = sqlx::query_as!(DbMessage, "SELECT * FROM messages")
        .fetch_all(&mut *db).await;

    match query {
        Ok(q) => Template::render("list_messages", context!{
            title: "Messages",
            messages: q,
        }),
        Err(e) => Template::render("error", context!{
            title: "Error",
            error: e.to_string(),
        }),
    }
}

#[get("/message/<id>/json")]
async fn get_message_json<'r>(mut db: Connection<Db>, id: i64) -> Result<json::Json<DbMessage>> {
    let query = sqlx::query_as!(DbMessage, "SELECT * FROM messages WHERE id = ?", id)
        .fetch_one(&mut *db).await?;
    Ok(json::Json(query))
}

#[get("/message/<id>")]
async fn get_message<'r>(mut db: Connection<Db>, id: i64) -> Template {
    let query = sqlx::query_as!(DbMessage, "SELECT * FROM messages WHERE id = ?", id)
        .fetch_one(&mut *db).await;
    match query {
        Ok(q) => Template::render("list_messages", context!{
            title: "Message",
            messages: [q],
        }),
        Err(e) => Template::render("error", context!{
            title: "Error",
            error: e.to_string(),
        }),
    }
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    match Db::fetch(&rocket) {
        Some(db) => match sqlx::migrate!("./migrations").run(&**db).await {
            Ok(_) => Ok(rocket),
            Err(e) => {
                error!("Failed to initialize SQLx database: {}", e);
                Err(rocket)
            }
        }
        None => Err(rocket),
    }
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("SQLx Stage", |rocket| async {
        rocket.attach(Db::init())
            .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
            .mount("/db", routes![
                   message_submit_form, 
                   message_submit, 
                   get_message, 
                   get_message_json, 
                   list_messages, 
                   list_messages_json
            ])
            .attach(Template::fairing())
    })
}
