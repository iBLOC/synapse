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
use synapse::components::transformer::RecursiveCharacterTextSplitter;
use synapse::components::embedding::OpenAIEmbedding;
use synapse::components::rag::{
    InMemoryVectorStore, InMemoryIndexer, InMemoryRetriever,
    RetrieveOptions,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Set up components
    let embedder: Arc<dyn synapse::components::embedding::Embedding> = Arc::new(
        OpenAIEmbedding::new("text-embedding-3-small", "sk-..."),
    );
    let store = Arc::new(InMemoryVectorStore::new());
    let indexer = InMemoryIndexer::new(store.clone(), embedder.clone());
    let retriever = InMemoryRetriever::new(store.clone(), embedder.clone());

    // 2. Prepare documents
    let docs = vec![
        Document::text("rust-intro", include_str!("../data/rust_intro.md")),
        Document::text("ownership", include_str!("../data/ownership.md")),
        Document::text("borrowing", include_str!("../data/borrowing.md")),
    ];

    // 3. Chunk
    let splitter = RecursiveCharacterTextSplitter::new(1000, 200);
    let chunks = splitter.transform(docs).await?;

    println!("Created {} chunks", chunks.len());

    // 4. Index (embed + store)
    indexer.index(chunks).await?;
    println!("Indexed {} chunks", store.count().await);

    // 5. Query
    let query = "How does Rust's ownership system work?";
    let results = retriever.retrieve(
        query,
        RetrieveOptions {
            top_k: Some(3),
            threshold: Some(0.7),
        },
    ).await?;

    println!("\nTop {} relevant chunks:", results.len());
    for (i, doc) in results.iter().enumerate() {
        println!("\n[{}] ({}): {}", i + 1, doc.id, &doc.content[..200.min(doc.content.len())]);
    }

    // 6. Use as LLM context
    let context = results.iter()
        .map(|d| format!("Source: {}\n{}", d.id, d.content))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let prompt = format!(
        "Answer the question using the context below.\n\nContext:\n{}\n\nQuestion: {}\n\nAnswer:",
        context, query
    );

    // Use the LLM with this prompt...
    Ok(())
}
```

## Chunking Strategy

For different document types, choose the right splitter:

| Document Type | Recommended Splitter |
|---------------|---------------------|
| Long articles | `RecursiveCharacterTextSplitter` |
| Markdown docs | `MarkdownHeaderSplitter` |
| Code | `TokenTextSplitter` (or custom by function) |
| Short messages | No chunking (index directly) |

## Embedding Cost

OpenAI embedding costs ~$0.02 per 1M tokens. For 1000 documents of
~1000 tokens each, that's ~$0.02 to index. Reasonable for most use
cases.

## Tuning

### top_k
- **Low (1-3):** Focused, less context noise
- **High (10+):** More coverage, but may confuse LLM

### threshold
- **High (0.8+):** Only very similar chunks
- **Low (0.0):** All chunks (sorted by similarity)

### chunk_size
- **Small (200-500):** More focused, but may miss context
- **Large (2000+):** More context, but less precise retrieval

For most use cases, `chunk_size=1000, overlap=200, top_k=3-5` works well.

## Next Steps

- [MultiAgent Router](./multiagent-router.md) — Add multiple agents
- [Document & RAG concepts](../concepts/document-rag.md) — Deep dive
