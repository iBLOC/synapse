use std::collections::HashMap;

use synapse::components::document::Document;
use synapse::components::transformer::{DocumentTransformer, TokenTextSplitter};

fn make_doc(id: &str, content: &str) -> Document {
    Document {
        id: id.into(),
        content: content.into(),
        mime_type: "text/plain".into(),
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_token_splitter_short_text() {
    let splitter = TokenTextSplitter::new(100, 10);
    let doc = make_doc("doc1", "Short text");
    let chunks = splitter.transform(vec![doc]).await.unwrap();
    assert_eq!(chunks.len(), 1);
}

#[tokio::test]
async fn test_token_splitter_long_text_splits_by_words() {
    let splitter = TokenTextSplitter::new(20, 5);
    let words: Vec<String> = (0..50).map(|i| format!("w{}", i)).collect();
    let text = words.join(" ");
    let chunks = splitter.transform(vec![make_doc("doc1", &text)]).await.unwrap();
    assert!(chunks.len() > 1, "expected multiple chunks, got {}", chunks.len());
    for chunk in &chunks {
        let word_count = chunk.content.split_whitespace().count();
        assert!(word_count <= 20, "chunk too large: {} words", word_count);
    }
}

#[tokio::test]
async fn test_token_splitter_metadata_preserved() {
    let splitter = TokenTextSplitter::new(10, 2);
    let text: Vec<String> = (0..30).map(|i| format!("word{}", i)).collect();
    let text = text.join(" ");
    let mut metadata = HashMap::new();
    metadata.insert("source".into(), serde_json::json!("test"));
    let doc = Document {
        id: "doc1".into(),
        content: text,
        mime_type: "text/plain".into(),
        metadata,
    };
    let chunks = splitter.transform(vec![doc]).await.unwrap();
    assert!(chunks.len() > 1);
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.metadata.get("source").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(chunk.metadata.get("source_id").and_then(|v| v.as_str()), Some("doc1"));
        assert_eq!(chunk.metadata.get("chunk_index").and_then(|v| v.as_u64()), Some(i as u64));
    }
}

#[tokio::test]
async fn test_token_splitter_name() {
    let splitter = TokenTextSplitter::new(100, 10);
    assert_eq!(splitter.name(), "TokenTextSplitter");
}
