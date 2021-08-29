use futures::stream::TryStreamExt;
use mongodb::bson::Document;
use mongodb::{Client, Collection};

pub struct FeedsRepository {
    collection: Collection<Document>,
}

impl FeedsRepository {
    pub fn new(client: &Client) -> FeedsRepository {
        let db = client.database("headlines");
        let feeds = db.collection::<Document>("feeds");

        FeedsRepository { collection: feeds }
    }

    pub async fn get_all(&self) -> Result<Vec<Document>, anyhow::Error> {
        let feeds_cursor = self.collection.find(None, None).await?;
        let feeds: Vec<Document> = feeds_cursor.try_collect().await?;

        Ok(feeds)
    }
}
