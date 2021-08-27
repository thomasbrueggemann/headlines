use mongodb::bson::{doc, Document};
use mongodb::{options::ClientOptions, Client};
use rss::Channel;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    let client_options =
        ClientOptions::parse("mongodb://headlines:senildeah@localhost:27017").await?;

    let client = Client::with_options(client_options)?;
    let db = client.database("headlines");
    let collection = db.collection::<Document>("headline_versions");

    let channel = read_feed().await?;

    for item in channel.items().iter() {
        let title = item.title().unwrap();
        let id = &item.guid().unwrap().value;

        println!("{}", title);

        let doc_title = doc! {
            "title": title,
            "changed": get_unix_seconds()
        };

        let doc_item = doc! {
            "_id": id,
            "titles": [doc_title],
            "feed": "tagesschau.de",
            "created":  get_unix_seconds()
        };

        collection.insert_one(doc_item, None).await?;
    }

    return Ok(());
}

async fn read_feed() -> Result<Channel, anyhow::Error> {
    let content = reqwest::get("https://www.tagesschau.de/xml/rss2/")
        .await?
        .bytes()
        .await?;

    let channel = Channel::read_from(&content[..])?;
    Ok(channel)
}

fn get_unix_seconds() -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    return since_the_epoch.as_secs() as i64;
}
