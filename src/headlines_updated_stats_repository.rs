use futures::stream::TryStreamExt;
use mongodb::bson::{doc, DateTime, Document};
use mongodb::{Client, Collection};

pub struct HeadlinesUpdatedStatsRepository {
    collection: Collection<Document>,
}

impl HeadlinesUpdatedStatsRepository {
    pub fn new(client: &Client) -> HeadlinesUpdatedStatsRepository {
        let db = client.database("headlines");
        let headlines_updated_stats = db.collection::<Document>("headlines_updated_stats");

        HeadlinesUpdatedStatsRepository {
            collection: headlines_updated_stats,
        }
    }

    pub async fn insert(&self, update_counter: i32, feed_id: &str, feed_locale: &str) {
        let stats_doc = doc! {
           "metadata": {
               "feed": feed_id,
               "locale": feed_locale
            },
           "timestamp": DateTime::now(),
           "updated": update_counter
        };

        self.collection.insert_one(stats_doc, None).await.unwrap();
    }

    pub async fn get_update_statistics_by_feed(
        &self,
        locale: &str,
    ) -> Result<Vec<Document>, anyhow::Error> {
        let pipeline = vec![
            doc! {
                "$match": {
                    "metadata.locale": locale
                }
            },
            doc! {
                "$group": {
                    "_id": "$metadata.feed",
                    "updates": {
                        "$sum": "$updated"
                    }
                }
            },
        ];

        let cursor = self.collection.aggregate(pipeline, None).await?;
        let documents: Vec<Document> = cursor.try_collect().await?;

        Ok(documents)
    }
}
