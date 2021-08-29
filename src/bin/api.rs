#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

use mongodb::bson::Document;
use mongodb::{options::ClientOptions, Client};
use rocket::serde::json::Json;
use rocket::serde::Serialize;

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
}

#[get("/headline/changes?<locale>")]
async fn headline_changes(locale: String) -> Json<Vec<HeadlineChangesResponse>> {
    let opts = ClientOptions::parse("mongodb://headlines:senildeah@localhost:27017")
        .await
        .unwrap();

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

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, headline_changes])
}
