use std::collections::HashMap;

use serde_json::json;
use synapse::components::document::Document;
use synapse::components::transformer::{CharacterTextSplitter, DocumentTransformer};

fn make_doc(id: &str, content: &str) -> Document {
    Document {
        id: id.into(),
        content: content.into(),
        mime_type: "text/plain".into(),
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_character_splitter_short_text_no_split() {
    let splitter = CharacterTextSplitter::new(1000, 200);
    let doc = make_doc("doc1", "Short text");
    let chunks = splitter.transform(vec![doc]).await.unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "Short text");
}

#[tokio::test]
async fn test_character_splitter_long_text_splits() {
    let splitter = CharacterTextSplitter::new(50, 10);
    let long = "a".repeat(200);
    let doc = make_doc("doc1", &long);
    let chunks = splitter.transform(vec![doc]).await.unwrap();
    assert!(chunks.len() > 1, "expected multiple chunks, got {}", chunks.len());
    for chunk in &chunks {
        assert!(chunk.content.len() <= 50, "chunk too large: {} chars", chunk.content.len());
    }
}

#[tokio::test]
async fn test_character_splitter_metadata_preserved() {
    let splitter = CharacterTextSplitter::new(50, 10);
    let mut metadata = HashMap::new();
    metadata.insert("source".into(), json!("test"));
    let doc = Document {
        id: "doc1".into(),
        content: "a".repeat(200),
        mime_type: "text/plain".into(),
        metadata,
    };
    let chunks = splitter.transform(vec![doc]).await.unwrap();
    assert!(chunks.len() > 1);
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.metadata.get("source").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(chunk.metadata.get("source_id").and_then(|v| v.as_str()), Some("doc1"));
        assert_eq!(chunk.metadata.get("chunk_index").and_then(|v| v.as_u64()), Some(i as u64));
        assert_eq!(chunk.id, format!("doc1_{}", i));
    }
}

#[tokio::test]
async fn test_character_splitter_name() {
    let splitter = CharacterTextSplitter::new(100, 10);
    assert_eq!(splitter.name(), "CharacterTextSplitter");
}
