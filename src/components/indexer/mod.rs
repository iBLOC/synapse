use async_trait::async_trait;
use crate::components::retriever::Document;

pub type IndexerResult<T> = Result<T, IndexerError>;

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("index error: {0}")]
    Index(String),
    #[error("collection not found: {0}")]
    CollectionNotFound(String),
    #[error("document too large")]
    DocumentTooLarge,
}

#[async_trait]
pub trait Indexer: Send + Sync {
    async fn index(&self, documents: Vec<Document>) -> IndexerResult<()>;
    
    async fn index_from_text(&self, texts: Vec<String>) -> IndexerResult<()>;
    
    async fn delete(&self, doc_ids: Vec<String>) -> IndexerResult<()>;
    
    async fn flush(&self) -> IndexerResult<()>;
}

pub struct IndexOptions {
    pub collection_name: String,
    pub batch_size: Option<usize>,
}

impl Default for IndexOptions {
    fn default() -> Self {
        Self {
            collection_name: "default".to_string(),
            batch_size: Some(100),
        }
    }
}
