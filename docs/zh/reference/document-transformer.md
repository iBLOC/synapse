---
title: DocumentTransformer
---

# 文档转换器

面向 RAG pipeline 的分块抽象。

## 定义

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

## 内置 Splitter

  - `CharacterTextSplitter` —— 固定字符数 + 重叠
  - `RecursiveCharacterTextSplitter` —— 递归分隔符
  - `MarkdownHeaderSplitter` —— 按 markdown header 切分
  - `TokenTextSplitter` —— 按近似 token 数切分

## 输出元数据

每个 chunk 携带：

  - `source_id` —— 原始文档 ID
  - `chunk_index` —— 在源文档中的序号

## 相关阅读
