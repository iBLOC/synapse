use std::collections::HashMap;

use synapse::components::retriever::Document;
use synapse::components::transformer::{DocumentTransformer, MarkdownHeaderSplitter};

fn make_doc(id: &str, content: &str) -> Document {
    Document {
        id: id.into(),
        content: content.into(),
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_markdown_splitter_basic() {
    let splitter = MarkdownHeaderSplitter::new();
    let md = "# Header 1\nFirst content.\n## Header 2\nSecond content.\n# Header 3\nThird content.";
    let chunks = splitter.transform(vec![make_doc("doc1", md)]).await.unwrap();
    assert_eq!(chunks.len(), 3, "expected 3 chunks, got {}: {:?}", chunks.len(), chunks);

    assert_eq!(chunks[0].metadata.get("h1").and_then(|v| v.as_str()), Some("Header 1"));
    assert!(chunks[0].content.contains("First content"));

    assert_eq!(chunks[1].metadata.get("h1").and_then(|v| v.as_str()), Some("Header 1"));
    assert_eq!(chunks[1].metadata.get("h2").and_then(|v| v.as_str()), Some("Header 2"));

    assert_eq!(chunks[2].metadata.get("h1").and_then(|v| v.as_str()), Some("Header 3"));
    assert_eq!(chunks[2].metadata.get("h2"), None);
}

#[tokio::test]
async fn test_markdown_splitter_name() {
    let splitter = MarkdownHeaderSplitter::new();
    assert_eq!(splitter.name(), "MarkdownHeaderSplitter");
}
