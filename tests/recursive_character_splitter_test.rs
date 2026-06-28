use std::collections::HashMap;

use synapse::components::retriever::Document;
use synapse::components::transformer::{DocumentTransformer, RecursiveCharacterTextSplitter};

fn make_doc(id: &str, content: &str) -> Document {
    Document {
        id: id.into(),
        content: content.into(),
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_recursive_splitter_splits_by_paragraph() {
    let splitter = RecursiveCharacterTextSplitter::new(40, 5);
    let text = "Paragraph 1 here.\n\nParagraph 2 here.\n\nParagraph 3 here.";
    let chunks = splitter.transform(vec![make_doc("doc1", text)]).await.unwrap();
    assert!(chunks.len() >= 2, "expected multiple chunks, got {}: {:?}", chunks.len(), chunks);
    for chunk in &chunks {
        assert!(chunk.content.len() <= 40, "chunk too large: {} chars", chunk.content.len());
    }
}

#[tokio::test]
async fn test_recursive_splitter_recursive_fallback() {
    let splitter = RecursiveCharacterTextSplitter::new(20, 5);
    let text = "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll";
    let chunks = splitter.transform(vec![make_doc("doc1", text)]).await.unwrap();
    assert!(chunks.len() > 1, "expected multiple chunks for one paragraph, got {}", chunks.len());
    for chunk in &chunks {
        assert!(chunk.content.len() <= 20, "chunk too large: {} chars", chunk.content.len());
    }
}

#[tokio::test]
async fn test_recursive_splitter_name() {
    let splitter = RecursiveCharacterTextSplitter::new(100, 10);
    assert_eq!(splitter.name(), "RecursiveCharacterTextSplitter");
}
