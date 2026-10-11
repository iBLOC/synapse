---
title: DocumentTransformer
---

# DocumentTransformer · 文档转换器

Chunking abstraction for RAG pipelines.
*面向 RAG pipeline 的分块抽象。*

## Definition · 定义

```rust
#[async_trait]
pub trait DocumentTransformer: Send + Sync {
    async fn transform(
        &self,
        documents: Vec<Document>,
    ) -> DocumentResult<Vec<Document>>;
    fn name(&self) -> &str;
}
```

## Built-in Splitters · 内置 Splitter

- `CharacterTextSplitter` — Fixed char count + overlap
  *`CharacterTextSplitter` —— 固定字符数 + 重叠*
- `RecursiveCharacterTextSplitter` — Recursive separator
  *`RecursiveCharacterTextSplitter` —— 递归分隔符*
- `MarkdownHeaderSplitter` — By markdown headers
  *`MarkdownHeaderSplitter` —— 按 markdown header 切分*
- `TokenTextSplitter` — By approximate token count
  *`TokenTextSplitter` —— 按近似 token 数切分*

## Output Metadata · 输出元数据

Each chunk carries:
*每个 chunk 携带：*

- `source_id` — original document ID
  *`source_id` —— 原始文档 ID*
- `chunk_index` — sequence in source
  *`chunk_index` —— 在源文档中的序号*

## See Also · 相关阅读

- [Document & RAG concepts](/concepts/document-rag)
- [RAG Pipeline how-to](/howto/rag-pipeline)
