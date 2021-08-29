use futures::stream::TryStreamExt;
use mongodb::bson::{doc, DateTime, Document};
use mongodb::options::FindOptions;
use mongodb::{Client, Collection};

pub struct HeadlineVersionsRepository {
    collection: Collection<Document>,
}

impl HeadlineVersionsRepository {
    pub fn new(client: &Client) -> HeadlineVersionsRepository {
        let db = client.database("headlines");
        let headline_versions = db.collection::<Document>("headline_versions");

        HeadlineVersionsRepository {
            collection: headline_versions,
        }
    }

    pub async fn get_by_ids(&self, ids: Vec<String>) -> Result<Vec<Document>, anyhow::Error> {
        let filter_ids = doc! { "_id": { "$in": &ids } };
        let cursor = self.collection.find(filter_ids, None).await?;
        let documents: Vec<Document> = cursor.try_collect().await?;

        Ok(documents)
    }

    pub async fn get(&self, locale: &str) -> Result<Vec<Document>, anyhow::Error> {
        let filter_ids = doc! { "title_changed": true, "locale": locale };

        let find_options = FindOptions::builder()
            .sort(doc! { "changed": -1 })
            .limit(100)
            .build();

        let cursor = self.collection.find(filter_ids, find_options).await?;
        let documents: Vec<Document> = cursor.try_collect().await?;

        Ok(documents)
    }

    pub async fn title_changed(&self, id: &str, title: &str) {
        let doc_title = doc! {
            "title": title,
            "changed": DateTime::now()
        };

        let md5_title = format!("{:x}", md5::compute(title));

        let update = doc! {
            "$set": {
                "latest_title_hash": md5_title,
                "title_changed": true,
                "changed": DateTime::now()
            },
            "$unset": {
                "no_change_expiry": ""
            },
            "$push": {
                "titles": doc_title
            }
        };

        let update_query = doc! { "_id": id };

        self.collection
            .update_one(update_query, update, None)
            .await
            .unwrap();
    }

    pub async fn insert(
        &self,
        id: &str,
        title: &str,
        link: &str,
        feed_id: &str,
        feed_locale: &str,
    ) {
        let doc_title = doc! {
            "title": title,
            "changed": DateTime::now()
        };

        let md5_title = format!("{:x}", md5::compute(title));

        let doc_item = doc! {
            "_id": id,
            "titles": [doc_title],
            "latest_title_hash": md5_title,
            "feed": feed_id,
            "created": DateTime::now(),
            "changed": DateTime::now(),
            "title_changed": false,
            "link": link,
            "locale": feed_locale,
            "no_change_expiry": DateTime::now()
        };

        self.collection.insert_one(doc_item, None).await.unwrap();
    }
}
