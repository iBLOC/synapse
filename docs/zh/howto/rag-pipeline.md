---
title: Build a RAG Pipeline
---

# 构建 RAG Pipeline

检索增强生成（RAG）：让 LLM 的回答扎根于你自己的数据。

## 目标

索引文档，根据用户 query 检索相关 chunk，再把 chunk 作为 LLM 的上下文。

## 流程

```
Documents
  → [Chunking] → Chunks
  → [Embedding] → Vectors
  → [VectorStore] → Indexed
  → [Query → Retrieve] → Relevant chunks
  → [LLM + context] → Grounded answer
```

## 完整示例

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

## 分块策略

| 文档类型 | 推荐 Splitter |
|---------------|---------------------|
| 长文章 | `RecursiveCharacterTextSplitter` |
| Markdown 文档 | `MarkdownHeaderSplitter` |
| 代码 | `TokenTextSplitter` |
| 短消息 | 不分块（直接索引） |

## 调参

  - top_k：1-3（聚焦）或 10+（覆盖更广）
  - threshold：0.8+（只要相似的）或 0.0（全部）
  - chunk_size：200-500（聚焦）或 2000+（上下文更足）

默认值 `chunk_size=1000, overlap=200, top_k=3-5` 对多数场景够用。

## 下一步

  - 加入多个 agent
  - 深入阅读
