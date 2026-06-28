# DocumentTransformer

Chunking abstraction for RAG pipelines.

## Definition

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

## Built-in Splitters

- `CharacterTextSplitter` — Fixed char count + overlap
- `RecursiveCharacterTextSplitter` — Recursive separator
- `MarkdownHeaderSplitter` — By markdown headers
- `TokenTextSplitter` — By approximate token count

## Output Metadata

Each chunk carries:
- `source_id` — original document ID
- `chunk_index` — sequence in source

## See Also

- [Document & RAG concepts](../concepts/document-rag.md)
- [RAG Pipeline how-to](../howto/rag-pipeline.md)
