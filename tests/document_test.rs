use synapse::components::document::Document;

#[test]
fn test_document_new_with_mime_type() {
    let doc = Document::new("id1", "content", "text/html");
    assert_eq!(doc.id, "id1");
    assert_eq!(doc.content, "content");
    assert_eq!(doc.mime_type, "text/html");
    assert!(doc.metadata.is_empty());
}

#[test]
fn test_document_text_default_mime_type() {
    let doc = Document::text("id1", "content");
    assert_eq!(doc.id, "id1");
    assert_eq!(doc.content, "content");
    assert_eq!(doc.mime_type, "text/plain");
}

#[test]
fn test_document_with_metadata() {
    let doc = Document::text("id1", "content")
        .with_metadata("source", serde_json::json!("file.txt"))
        .with_metadata("chunk", serde_json::json!(0));
    assert_eq!(doc.metadata.len(), 2);
    assert_eq!(
        doc.metadata.get("source").and_then(|v| v.as_str()),
        Some("file.txt")
    );
}

#[test]
fn test_retriever_document_is_document() {
    use synapse::components::retriever::Document as RetrieverDocument;
    let d1: Document = Document::text("x", "y");
    let d2: RetrieverDocument = d1.clone();
    assert_eq!(d1.mime_type, d2.mime_type);
    assert_eq!(d1.id, d2.id);
}
