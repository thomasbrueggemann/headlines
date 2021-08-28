use futures::stream::TryStreamExt;
use mongodb::bson::{doc, DateTime, Document};
use mongodb::Collection;
use mongodb::{options::ClientOptions, Client};
use rss::{Channel, Item};
use std::collections::HashMap;

mod HeadlineVersionsRepository;

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    let opts = ClientOptions::parse("mongodb://headlines:senildeah@localhost:27017").await?;

    let client = Client::with_options(opts)?;
    let db = client.database("headlines");
    let feeds = db.collection::<Document>("feeds");
    let headlines_updated_stats = db.collection::<Document>("headlines_updated_stats");

    let headline_version_repository = HeadlineVersionsRepository::new(&client);

    let feeds_cursor = feeds.find(None, None).await?;
    let feeds: Vec<Document> = feeds_cursor.try_collect().await?;

    for feed in feeds.iter() {
        let feed_url = feed.get_str("rss").unwrap();
        let feed_id = feed.get_str("_id").unwrap();

        println!("# PARSE FEED {}", feed_id);

        let channel = read_feed(feed_url).await?;

        let items_by_id =
            get_item_by_id_lookup(feed_id, channel.items(), &headline_version_repository).await?;

        let mut update_counter: i32 = 0;

        for item in channel.items().iter() {
            if item.title().is_none() || item.link().is_none() || item.guid().is_none() {
                continue;
            }

            let title = item.title().unwrap();
            let link = item.link().unwrap();

            let id = generate_id(feed.get_str("_id").unwrap(), &item.guid().unwrap().value);

            let doc_title = doc! {
                "title": title,
                "changed": DateTime::now()
            };

            let md5_title = format!("{:x}", md5::compute(title));

            if items_by_id.contains_key(&id) {
                let stored_title_md5 = items_by_id
                    .get(&id)
                    .unwrap()
                    .get_str("latest_title_hash")
                    .unwrap();

                let update_query = doc! { "_id": &id };

                if md5_title != stored_title_md5 {
                    println!("~ {}", &id);

                    headline_version_repository
                        .title_changed(&id, &title)
                        .await?;

                    update_counter += 1;
                }
            } else {
                println!("+ {}", &id);

                headline_version_repository
                    .insert(
                        &id,
                        &title,
                        &link,
                        &feed_url,
                        feed.get_str("locale").unwrap(),
                    )
                    .await?;
            }
        }

        if update_counter > 0 {
            let stats_doc = doc! {
               "metadata": [{"feed": feed_id}, {"locale": feed.get_str("locale").unwrap()}],
               "timestamp": DateTime::now(),
               "updated": update_counter
            };

            headlines_updated_stats.insert_one(stats_doc, None).await?;
        }
    }

    Ok(())
}

async fn read_feed(feed_url: &str) -> Result<Channel, anyhow::Error> {
    let content = reqwest::get(feed_url).await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;

    Ok(channel)
}

async fn get_item_by_id_lookup(
    feed_id: &str,
    items: &[Item],
    headline_versions_repository: &HeadlineVersionsRepository,
) -> Result<HashMap<String, Document>, anyhow::Error> {
    let ids: Vec<String> = items
        .iter()
        .filter(|item| item.guid().is_some())
        .map(|item| generate_id(feed_id, &item.guid().unwrap().value))
        .collect();

    let mut items_by_id: HashMap<String, Document> = HashMap::new();
    let documents: Vec<Document> = headline_versions_repository.get_by_ids(ids).await?;

    for document in documents.iter() {
        items_by_id.insert(
            document.get_str("_id").unwrap().to_string(),
            document.to_owned(),
        );
    }

    Ok(items_by_id)
}

fn generate_id(feed_id: &str, guid: &str) -> String {
    let raw_id = format!("{}@{}", feed_id, guid);
    format!("{:x}", md5::compute(raw_id))
}
