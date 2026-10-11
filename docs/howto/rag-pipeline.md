---
title: Build a RAG Pipeline
---

# Build a RAG Pipeline · 构建 RAG Pipeline

Retrieval-Augmented Generation: ground LLM responses in your own data.
*检索增强生成（RAG）：让 LLM 的回答扎根于你自己的数据。*

## Goal · 目标

Index documents, retrieve relevant chunks based on user query, use
chunks as context for the LLM.
*索引文档，根据用户 query 检索相关 chunk，再把 chunk 作为 LLM 的上下文。*

## Pipeline · 流程

```
Documents
  → [Chunking] → Chunks
  → [Embedding] → Vectors
  → [VectorStore] → Indexed
  → [Query → Retrieve] → Relevant chunks
  → [LLM + context] → Grounded answer
```

## Full Example · 完整示例

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
    let embedder: Arc<dyn synapse::components::embedding::Embedding> = Arc::new(
        OpenAIEmbedding::new("text-embedding-3-small", "sk-..."),
    );
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

## Chunking Strategy · 分块策略

| Document Type / 文档类型 | Recommended Splitter / 推荐 Splitter |
|---------------|---------------------|
| Long articles / 长文章 | `RecursiveCharacterTextSplitter` |
| Markdown docs / Markdown 文档 | `MarkdownHeaderSplitter` |
| Code / 代码 | `TokenTextSplitter` |
| Short messages / 短消息 | No chunking (index directly) / 不分块（直接索引） |

## Tuning · 调参

- **top_k**: 1-3 (focused) or 10+ (more coverage)
  *top_k：1-3（聚焦）或 10+（覆盖更广）*
- **threshold**: 0.8+ (only similar) or 0.0 (all)
  *threshold：0.8+（只要相似的）或 0.0（全部）*
- **chunk_size**: 200-500 (focused) or 2000+ (more context)
  *chunk_size：200-500（聚焦）或 2000+（上下文更足）*

Default: `chunk_size=1000, overlap=200, top_k=3-5` works for most.
*默认值 `chunk_size=1000, overlap=200, top_k=3-5` 对多数场景够用。*

## Next Steps · 下一步

- [MultiAgent Router](/howto/multiagent-router) — Add multiple agents
  *加入多个 agent*
- [Document & RAG concepts](/concepts/document-rag) — Deep dive
  *深入阅读*
