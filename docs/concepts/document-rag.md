---
title: Document & RAG
---

# Document & RAG · 文档与 RAG

Retrieval-Augmented Generation (RAG) is a common pattern: given a user
query, retrieve relevant documents from a knowledge base, and use them
as context for the LLM to generate a grounded answer.
*检索增强生成（RAG）是一种常见模式：给定用户 query，从知识库中检索相关文档，再将其作为 context 交给 LLM，生成有依据的回答。*

## Pipeline · 流水线

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

## Document Type · 文档类型

Unified document type:
*统一的文档类型：*

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

## Parsers · 解析器

```rust
pub trait DocumentParser: Send + Sync {
    fn supported_mime_types(&self) -> Vec<&'static str>;
    async fn parse(&self, content: &[u8], mime_type: &str)
        -> DocumentResult<ParsedDocument>;
}
```

Built-in:
*内置实现：*
- `TextParser` — text/plain, text/markdown, text/html
- `JsonParser` — application/json

## Chunking (DocumentTransformer) · 分块（DocumentTransformer）

```rust
#[async_trait]
pub trait DocumentTransformer: Send + Sync {
    async fn transform(&self, documents: Vec<Document>)
        -> DocumentResult<Vec<Document>>;
    fn name(&self) -> &str;
}
```

Built-in splitters:
*内置 splitter：*

| Splitter / 分块器 | Behavior / 行为 |
|----------|----------|
| `CharacterTextSplitter` | Fixed char count with overlap / 固定字符数 + 重叠 |
| `RecursiveCharacterTextSplitter` | Recursive separator (`\n\n` → `\n` → ` `) / 递归分隔符（`\n\n` → `\n` → 空格） |
| `MarkdownHeaderSplitter` | By markdown headers (h1/h2/h3) / 按 markdown 标题（h1/h2/h3） |
| `TokenTextSplitter` | By approximate token count / 按近似 token 数 |

Each chunk carries:
*每个 chunk 携带：*
- `source_id` — original document ID
  *`source_id` —— 源文档 ID*
- `chunk_index` — sequence in source
  *`chunk_index` —— 在源文档中的序号*
- `mime_type` — inherited from source
  *`mime_type` —— 继承自源文档*
- Original metadata preserved
  *原始 metadata 保留*

## Embedding · Embedding（嵌入）

```rust
#[async_trait]
pub trait Embedding: Send + Sync {
    async fn embed(&self, texts: Vec<String>) -> EmbeddingResult<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
    fn name(&self) -> &str;
}
```

## VectorStore + Indexer · 向量存储与索引

```rust
pub trait Indexer: Send + Sync {
    async fn index(&self, documents: Vec<Document>) -> IndexerResult<()>;
    async fn index_from_text(&self, texts: Vec<String>) -> IndexerResult<()>;
    async fn delete(&self, doc_ids: Vec<String>) -> IndexerResult<()>;
    async fn flush(&self) -> IndexerResult<()>;
}
```

## Retriever · 检索器

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

## Full RAG Example · 完整 RAG 示例

```rust
use std::sync::Arc;
use synapse::components::document::Document;
use synapse::components::transformer::RecursiveCharacterTextSplitter;
use synapse::components::embedding::OpenAIEmbedding;
use synapse::components::rag::{InMemoryVectorStore, InMemoryIndexer, InMemoryRetriever, RetrieveOptions};

let splitter = RecursiveCharacterTextSplitter::new(1000, 200);
let embedder: Arc<dyn synapse::components::embedding::Embedding> = Arc::new(
    OpenAIEmbedding::new("text-embedding-3-small", "sk-..."),
);

let store = Arc::new(InMemoryVectorStore::new());
let indexer = InMemoryIndexer::new(store.clone(), embedder.clone());
let retriever = InMemoryRetriever::new(store.clone(), embedder.clone());

let docs = vec![
    Document::text("doc1", long_text_1),
    Document::text("doc2", long_text_2),
];

let chunks = splitter.transform(docs).await?;
indexer.index(chunks).await?;

let results = retriever
    .retrieve("user question", RetrieveOptions {
        top_k: Some(3),
        threshold: Some(0.7),
    })
    .await?;
```

See [RAG Pipeline how-to](/howto/rag-pipeline) for a full walkthrough.
*完整流程见 [RAG Pipeline how-to](/howto/rag-pipeline) 文档。*
