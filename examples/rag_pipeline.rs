//! Full RAG pipeline: chunk documents, embed, index into an in-memory
//! vector store, then retrieve by cosine similarity.
//!
//! Run:
//!   OPENAI_API_KEY=sk-... cargo run --example rag_pipeline

use std::sync::Arc;

use synapse::components::document::Document;
use synapse::components::embedding::openai::OpenAIEmbedding;
use synapse::components::embedding::Embedding;
use synapse::components::indexer::Indexer;
use synapse::components::rag::{
    InMemoryIndexer, InMemoryRetriever, InMemoryVectorStore,
};
use synapse::components::retriever::{RetrieveOptions, Retriever};
use synapse::components::transformer::{
    DocumentTransformer, RecursiveCharacterTextSplitter,
};
use synapse::transport::ReqwestHttpClient;

const RUST_INTRO: &str = "Rust is a systems programming language focused on \
safety, speed, and concurrency. It enforces memory safety without a garbage \
collector through its ownership model.";

const OWNERSHIP: &str = "Ownership is Rust's central feature. Each value has \
a single owner; when the owner goes out of scope the value is dropped. \
Borrowing lets references access values without taking ownership.";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let embedder: Arc<dyn Embedding> = Arc::new(OpenAIEmbedding::new(
        Arc::new(ReqwestHttpClient::new()),
        "sk-...",
    ));
    let store = Arc::new(InMemoryVectorStore::new());
    let indexer = InMemoryIndexer::new(store.clone(), embedder.clone());
    let retriever = InMemoryRetriever::new(store.clone(), embedder.clone());

    let docs = vec![
        Document::text("rust-intro", RUST_INTRO),
        Document::text("ownership", OWNERSHIP),
    ];

    let splitter = RecursiveCharacterTextSplitter::new(1000, 200);
    let chunks = splitter.transform(docs).await?;

    indexer.index(chunks).await?;

    let results = retriever
        .retrieve(
            "How does Rust's ownership work?",
            RetrieveOptions {
                top_k: Some(3),
                threshold: Some(0.7),
            },
        )
        .await?;

    for doc in &results {
        println!("Source: {}\n{}\n", doc.id, doc.content);
    }

    Ok(())
}
