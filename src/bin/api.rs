#[macro_use]
extern crate rocket;

use mongodb::bson::Document;
use mongodb::{options::ClientOptions, Client};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::{Request, Response};
use std::env;

use headlines::headline_versions_repository::HeadlineVersionsRepository;

#[get("/")]
fn index() -> &'static str {
    "Headlines v1.0.0"
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct HeadlineChangesResponse {
    changed_title: String,
    original_title: String,
    link: String,
    changed: i64,
    created: i64,
    feed: String,
}

#[get("/headline/changes?<locale>")]
async fn headline_changes(locale: String) -> Json<Vec<HeadlineChangesResponse>> {
    let connection_string = env::var("MONGO_CONNECTION_STRING").unwrap();
    let opts = ClientOptions::parse(connection_string).await.unwrap();

    let client = Client::with_options(opts).unwrap();
    let headline_versions_repo = HeadlineVersionsRepository::new(&client);
    let headline_versions = headline_versions_repo.get(&locale).await.unwrap();

    let response_items: Vec<HeadlineChangesResponse> = headline_versions
        .iter()
        .map(|headline_version| {
            let mut titles: Vec<Document> = headline_version
                .get_array("titles")
                .unwrap()
                .to_owned()
                .iter()
                .map(|doc| doc.as_document().unwrap().to_owned())
                .collect();

            titles.sort_by(|a, b| {
                b.get_datetime("changed")
                    .unwrap()
                    .cmp(&a.get_datetime("changed").unwrap())
            });

            HeadlineChangesResponse {
                changed_title: get_title_string(titles.first()),
                original_title: get_title_string(titles.last()),
                link: headline_version.get_str("link").unwrap().to_string(),
                created: get_timestamp_seconds(headline_version, "created"),
                changed: get_timestamp_seconds(headline_version, "changed"),
                feed: headline_version.get_str("feed").unwrap().to_string(),
            }
        })
        .collect();

    Json(response_items)
}

fn get_title_string(doc: Option<&Document>) -> String {
    doc.unwrap().get_str("title").unwrap().to_string()
}

fn get_timestamp_seconds(doc: &Document, key: &str) -> i64 {
    doc.get_datetime(key).unwrap().timestamp_millis() / 1000
}

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _req: &'r Request<'_>, res: &mut Response<'r>) {
        res.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        res.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        res.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        res.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(CORS)
        .mount("/", routes![index, headline_changes])
}
