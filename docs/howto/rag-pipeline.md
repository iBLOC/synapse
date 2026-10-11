---
title: Build a RAG Pipeline
---

# Build a RAG Pipeline

Retrieval-Augmented Generation: ground LLM responses in your own data.

## Goal

Index documents, retrieve relevant chunks based on user query, use
chunks as context for the LLM.

## Pipeline

```
Documents
  → [Chunking] → Chunks
  → [Embedding] → Vectors
  → [VectorStore] → Indexed
  → [Query → Retrieve] → Relevant chunks
  → [LLM + context] → Grounded answer
```

## Full Example

```rust
use std::sync::Arc;
use synapse::components::document::Document;
use synapse::components::embedding::openai::OpenAIEmbedding;
use synapse::components::embedding::Embedding;
use synapse::components::indexer::Indexer;
use synapse::components::rag::{
    InMemoryVectorStore, InMemoryIndexer, InMemoryRetriever,
};
use synapse::components::retriever::{RetrieveOptions, Retriever};
use synapse::components::transformer::{
    DocumentTransformer, RecursiveCharacterTextSplitter,
};
use synapse::transport::ReqwestHttpClient;

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
        Document::text("rust-intro", rust_intro_text),
        Document::text("ownership", ownership_text),
    ];

    let splitter = RecursiveCharacterTextSplitter::new(1000, 200);
    let chunks = splitter.transform(docs).await?;

    indexer.index(chunks).await?;

    let results = retriever.retrieve(
        "How does Rust's ownership work?",
        RetrieveOptions {
            top_k: Some(3),
            threshold: Some(0.7),
        },
    ).await?;

    let context = results.iter()
        .map(|d| format!("Source: {}\n{}", d.id, d.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    Ok(())
}
```

## Chunking Strategy

| Document Type | Recommended Splitter |
|---------------|---------------------|
| Long articles | `RecursiveCharacterTextSplitter` |
| Markdown docs | `MarkdownHeaderSplitter` |
| Code | `TokenTextSplitter` |
| Short messages | No chunking (index directly) |

## Tuning

- **top_k**: 1-3 (focused) or 10+ (more coverage)
- **threshold**: 0.8+ (only similar) or 0.0 (all)
- **chunk_size**: 200-500 (focused) or 2000+ (more context)

Default: `chunk_size=1000, overlap=200, top_k=3-5` works for most.

## Next Steps

- [MultiAgent Router](/howto/multiagent-router) — Add multiple agents
- [Document & RAG concepts](/concepts/document-rag) — Deep dive
