use mongodb::bson::Document;
use mongodb::{options::ClientOptions, Client};
use rss::{Channel, Item};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

use headlines::{
    feeds_repository::FeedsRepository, headline_versions_repository::HeadlineVersionsRepository,
    headlines_updated_stats_repository::HeadlinesUpdatedStatsRepository,
};

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    loop {
        let opts = ClientOptions::parse("mongodb://headlines:senildeah@localhost:27017").await?;
        let client = Client::with_options(opts)?;

        let headline_versions_repo = HeadlineVersionsRepository::new(&client);
        let feeds_repo = FeedsRepository::new(&client);
        let headlines_updated_stats_repo = HeadlinesUpdatedStatsRepository::new(&client);

        let feeds = feeds_repo.get_all().await?;
        for feed in feeds.iter() {
            let feed_url = feed.get_str("rss").unwrap();
            let feed_id = feed.get_str("_id").unwrap();
            let feed_locale = feed.get_str("locale").unwrap();

            println!("# PARSE FEED {}", feed_id);
            let channel = read_feed(feed_url).await?;
            let mut update_counter: i32 = 0;

            let items_by_id =
                get_item_by_id_lookup(feed_id, channel.items(), &headline_versions_repo).await?;

            for item in channel.items().iter() {
                if item.title().is_none() || item.link().is_none() || item.guid().is_none() {
                    continue;
                }

                let title = item.title().unwrap();
                let md5_title = format!("{:x}", md5::compute(title));

                let link = item.link().unwrap();
                let id = generate_id(feed.get_str("_id").unwrap(), &item.guid().unwrap().value);

                if items_by_id.contains_key(&id) {
                    let stored_title_md5 = items_by_id
                        .get(&id)
                        .unwrap()
                        .get_str("latest_title_hash")
                        .unwrap();

                    if md5_title != stored_title_md5 {
                        println!("~ {}", &id);
                        headline_versions_repo.title_changed(&id, &title).await;
                        update_counter += 1;
                    }
                } else {
                    println!("+ {}", &id);

                    headline_versions_repo
                        .insert(&id, &title, &link, &feed_url, &feed_locale)
                        .await;
                }
            }

            if update_counter > 0 {
                headlines_updated_stats_repo
                    .insert(update_counter, &feed_id, &feed_locale)
                    .await;
            }
        }

        println!("Wait 5 minutes till next execution...");
        sleep(Duration::from_secs(300)).await;
    }
}

async fn read_feed(feed_url: &str) -> Result<Channel, anyhow::Error> {
    let content = reqwest::get(feed_url).await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;

    Ok(channel)
}

async fn get_item_by_id_lookup(
    feed_id: &str,
    items: &[Item],
    headline_versions_repo: &HeadlineVersionsRepository,
) -> Result<HashMap<String, Document>, anyhow::Error> {
    let ids: Vec<String> = items
        .iter()
        .filter(|item| item.guid().is_some())
        .map(|item| generate_id(feed_id, &item.guid().unwrap().value))
        .collect();

    let mut items_by_id: HashMap<String, Document> = HashMap::new();
    let documents: Vec<Document> = headline_versions_repo.get_by_ids(ids).await?;

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
