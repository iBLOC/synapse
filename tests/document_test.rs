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

#[test]
fn test_document_to_json_roundtrip() {
    let doc = Document::text("doc1", "Hello, world!")
        .with_metadata("source", serde_json::json!("file.txt"))
        .with_metadata("chunk", serde_json::json!(0));
    let json = doc.to_json().expect("serialize failed");
    let parsed = Document::from_json(&json).expect("parse failed");
    assert_eq!(parsed.id, doc.id);
    assert_eq!(parsed.content, doc.content);
    assert_eq!(parsed.mime_type, doc.mime_type);
    assert_eq!(parsed.metadata.get("source"), doc.metadata.get("source"));
    assert_eq!(parsed.metadata.get("chunk"), doc.metadata.get("chunk"));
}

#[test]
fn test_document_json_pretty() {
    let doc = Document::text("d", "c");
    let pretty = doc.to_json_pretty().expect("pretty serialize failed");
    assert!(pretty.contains("\"id\""));
    assert!(pretty.contains("\"content\""));
    assert!(pretty.contains("\"mime_type\""));
    assert!(pretty.contains('\n'));
}

#[test]
fn test_document_json_vec_roundtrip() {
    let doc = Document::new("d", "c", "text/html");
    let bytes = doc.to_json_vec().expect("vec serialize failed");
    let parsed = Document::from_json_vec(&bytes).expect("vec parse failed");
    assert_eq!(parsed.id, doc.id);
    assert_eq!(parsed.mime_type, "text/html");
}

#[test]
fn test_document_json_complex_metadata() {
    use std::collections::HashMap;
    let mut metadata = HashMap::new();
    metadata.insert("nested".into(), serde_json::json!({"a": [1, 2, 3]}));
    metadata.insert("arr".into(), serde_json::json!(["x", "y"]));
    metadata.insert("num".into(), serde_json::json!(42.5));
    let doc = Document {
        id: "d".into(),
        content: "c".into(),
        mime_type: "application/json".into(),
        metadata,
    };
    let json = doc.to_json().unwrap();
    let parsed = Document::from_json(&json).unwrap();
    assert_eq!(parsed.metadata.get("nested").unwrap()["a"][2], 3);
    assert_eq!(parsed.metadata.get("num").unwrap().as_f64(), Some(42.5));
}
