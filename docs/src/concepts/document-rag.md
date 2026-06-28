# Document & RAG

Retrieval-Augmented Generation (RAG) is a common pattern: given a user
query, retrieve relevant documents from a knowledge base, and use
them as context for the LLM to generate a grounded answer.

## Pipeline

```
Raw bytes (PDF, HTML, text)
   ↓
[DocumentParser] → ParsedDocument
   ↓
Wrap as Document (id, content, mime_type, metadata)
   ↓
[DocumentTransformer] (chunking) → Vec<Document>
   ↓
[Embedding] (OpenAI / others) → Vec<Vec<f32>>
   ↓
[Indexer] → VectorStore
   ↓
[Retriever] (cosine similarity) → Vec<Document>
   ↓
Use as context for LLM
```

## Document Type

Unified document type (single canonical):

```rust
pub struct Document {
    pub id: String,
    pub content: String,
    pub mime_type: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Document {
    pub fn new(id, content, mime_type) -> Self;
    pub fn text(id, content) -> Self;  // defaults to text/plain
    pub fn with_metadata(self, key, value) -> Self;
    pub fn to_json(&self) -> Result<String, serde_json::Error>;
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error>;
}
```

## Parsers

```rust
pub trait DocumentParser: Send + Sync {
    fn supported_mime_types(&self) -> Vec<&'static str>;
    async fn parse(&self, content: &[u8], mime_type: &str)
        -> DocumentResult<ParsedDocument>;
}
```

Built-in:
- `TextParser` — text/plain, text/markdown, text/html
- `JsonParser` — application/json

## Chunking (DocumentTransformer)

```rust
#[async_trait]
pub trait DocumentTransformer: Send + Sync {
    async fn transform(&self, documents: Vec<Document>)
        -> DocumentResult<Vec<Document>>;
    fn name(&self) -> &str;
}
```

Built-in splitters:

| Splitter | Behavior |
|----------|----------|
| `CharacterTextSplitter` | Fixed char count with overlap |
| `RecursiveCharacterTextSplitter` | Recursive separator (`\n\n` → `\n` → ` `) |
| `MarkdownHeaderSplitter` | By markdown headers (h1/h2/h3) |
| `TokenTextSplitter` | By approximate token count |

Each chunk carries:
- `source_id` — original document ID
- `chunk_index` — sequence in source
- `mime_type` — inherited from source
- Original metadata preserved

## Embedding

```rust
#[async_trait]
pub trait Embedding: Send + Sync {
    async fn embed(&self, texts: Vec<String>) -> EmbeddingResult<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
    fn name(&self) -> &str;
}
```

Built-in:
- `OpenAIEmbedding` — text-embedding-3-small/large

## VectorStore + Indexer

```rust
pub trait Indexer: Send + Sync {
    async fn index(&self, documents: Vec<Document>) -> IndexerResult<()>;
    async fn index_from_text(&self, texts: Vec<String>) -> IndexerResult<()>;
    async fn delete(&self, doc_ids: Vec<String>) -> IndexerResult<()>;
    async fn flush(&self) -> IndexerResult<()>;
}
```

`InMemoryVectorStore` — cosine similarity, threshold filtering, in-process.

## Retriever

```rust
pub trait Retriever: Send + Sync {
    async fn retrieve(&self, query: &str, options: RetrieveOptions)
        -> RetrieverResult<Vec<Document>>;
}

pub struct RetrieveOptions {
    pub top_k: Option<usize>,
    pub threshold: Option<f32>,
}
```

## Full RAG Example

```rust
use std::sync::Arc;
use synapse::components::document::Document;
use synapse::components::transformer::RecursiveCharacterTextSplitter;
use synapse::components::embedding::OpenAIEmbedding;
use synapse::components::rag::{InMemoryVectorStore, InMemoryIndexer, InMemoryRetriever};
use synapse::components::rag::RetrieveOptions;

let splitter = RecursiveCharacterTextSplitter::new(1000, 200);
let embedder: Arc<dyn synapse::components::embedding::Embedding> = Arc::new(
    OpenAIEmbedding::new("text-embedding-3-small", "sk-..."),
);

let store = Arc::new(InMemoryVectorStore::new());
let indexer = InMemoryIndexer::new(store.clone(), embedder.clone());
let retriever = InMemoryRetriever::new(store.clone(), embedder.clone());

// 1. Prepare documents
let docs = vec![
    Document::text("doc1", long_text_1),
    Document::text("doc2", long_text_2),
];

// 2. Chunk
let chunks: Vec<Document> = splitter.transform(docs).await?;

// 3. Index (embed + store)
indexer.index(chunks).await?;

// 4. Query
let results = retriever
    .retrieve("user question", RetrieveOptions {
        top_k: Some(4),
        threshold: Some(0.7),
    })
    .await?;

// 5. Use as LLM context
let context = results.iter()
    .map(|d| d.content.clone())
    .collect::<Vec<_>>()
    .join("\n\n");
```

See [RAG Pipeline how-to](../howto/rag-pipeline.md) for a full walkthrough.
