//! Document transformers for RAG pre-processing.
//!
//! A `DocumentTransformer` takes documents and produces documents. The most
//! common use case is chunking long documents into smaller pieces suitable
//! for embedding and retrieval.

pub mod character_splitter;
pub mod markdown_header_splitter;
pub mod recursive_character_splitter;

pub use character_splitter::CharacterTextSplitter;
pub use recursive_character_splitter::RecursiveCharacterTextSplitter;

use async_trait::async_trait;

use crate::components::document::DocumentResult;
use crate::components::retriever::Document;

#[async_trait]
pub trait DocumentTransformer: Send + Sync {
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>>;
    fn name(&self) -> &str;
}
