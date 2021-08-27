use futures::stream::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::{options::ClientOptions, Client};
use rss::Channel;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    let opts = ClientOptions::parse("mongodb://headlines:senildeah@localhost:27017").await?;

    let client = Client::with_options(opts)?;
    let db = client.database("headlines");
    let headline_versions = db.collection::<Document>("headline_versions");
    let feeds = db.collection::<Document>("feeds");

    let mut feeds_cursor = feeds.find(doc! {}, None).await?;

    while let Some(feed) = feeds_cursor.try_next().await? {
        let feed_url = feed.get_str("rss").unwrap();
        println!("# PARSE FEED {}", feed_url);

        let channel = read_feed(feed_url).await?;
        let mut items_by_id: HashMap<String, Document> = HashMap::new();

        let ids: Vec<String> = channel
            .items()
            .iter()
            .filter(|item| item.guid().is_some())
            .map(|item| &item.guid().unwrap().value)
            .map(|guid| format!("{:x}", md5::compute(guid)))
            .collect();

        let filter_ids = doc! { "_id": { "$in": &ids } };
        println!("ids {}", ids.len());

        let mut headline_versions_cursor = headline_versions.find(filter_ids, None).await?;
        while let Some(item) = headline_versions_cursor.try_next().await? {
            items_by_id.insert(item.get_str("_id").unwrap().to_string(), item);
        }

        println!("ietms by id {}", items_by_id.len());

        for item in channel.items().iter() {
            if item.title().is_none() || item.link().is_none() || item.guid().is_none() {
                continue;
            }

            let title = item.title().unwrap();
            let link = item.link().unwrap();

            let id = item.link().unwrap();
            let id_hash = &format!("{:x}", md5::compute(id));

            let doc_title = doc! {
                "title": title,
                "changed": get_unix_seconds()
            };

            let md5_title = format!("{:x}", md5::compute(title));

            if items_by_id.contains_key(id_hash) {
                let stored_title_md5 = items_by_id
                    .get(id_hash)
                    .unwrap()
                    .get_str("latest_title_hash")
                    .unwrap();

                let update_query = doc! { "_id": link };

                if md5_title != stored_title_md5 {
                    let update = doc! {
                        "$set": {
                            "latest_title_hash": md5_title,
                            "title_changed": true
                        },
                        "$push": {
                            "titles": doc_title
                        }
                    };

                    println!("~ {}", title);
                    headline_versions
                        .update_one(update_query, update, None)
                        .await?;
                }
            } else {
                let doc_item = doc! {
                    "_id": id_hash,
                    "titles": [doc_title],
                    "latest_title_hash": md5_title,
                    "feed": "tagesschau.de",
                    "created":  get_unix_seconds(),
                    "title_changed": false,
                    "link": link
                };

                println!("+ {}", title);
                headline_versions.insert_one(doc_item, None).await?;
            }
        }
    }

    return Ok(());
}

async fn read_feed(feed_url: &str) -> Result<Channel, anyhow::Error> {
    let content = reqwest::get(feed_url).await?.bytes().await?;

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
