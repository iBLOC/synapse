use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::json;

use crate::components::document::DocumentResult;
use crate::components::retriever::Document;

use super::DocumentTransformer;

pub struct MarkdownHeaderSplitter {
    header_levels: Vec<String>,
}

impl MarkdownHeaderSplitter {
    pub fn new() -> Self {
        Self {
            header_levels: vec![
                "#".into(),
                "##".into(),
                "###".into(),
                "####".into(),
                "#####".into(),
                "######".into(),
            ],
        }
    }
}

impl Default for MarkdownHeaderSplitter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DocumentTransformer for MarkdownHeaderSplitter {
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>> {
        let mut result = Vec::new();
        for doc in documents {
            let segments = split_markdown(&doc.content, &self.header_levels);
            for (i, (headers, content)) in segments.into_iter().enumerate() {
                let mut metadata = doc.metadata.clone();
                for (level, text) in &headers {
                    metadata.insert(level.clone(), json!(text));
                }
                metadata.insert("source_id".into(), json!(doc.id));
                metadata.insert("chunk_index".into(), json!(i));
                result.push(Document {
                    id: format!("{}_{}", doc.id, i),
                    content,
                    mime_type: doc.mime_type.clone(),
                    metadata,
                });
            }
        }
        Ok(result)
    }

    fn name(&self) -> &str {
        "MarkdownHeaderSplitter"
    }
}

fn level_depth(level: &str) -> u32 {
    level.chars().filter(|c| *c == '#').count() as u32
}

fn normalize_level(level: &str) -> String {
    format!("h{}", level_depth(level))
}

fn key_depth(key: &str) -> u32 {
    key.strip_prefix('h')
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0)
}

fn split_markdown(
    text: &str,
    header_levels: &[String],
) -> Vec<(HashMap<String, String>, String)> {
    let mut chunks: Vec<(HashMap<String, String>, String)> = Vec::new();
    let mut current_headers: HashMap<String, String> = HashMap::new();
    let mut current_content = String::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        let matched_level = header_levels
            .iter()
            .find(|h| {
                let prefix = format!("{} ", h);
                trimmed.starts_with(&prefix)
            })
            .cloned();

        match matched_level {
            Some(level) => {
                if !current_content.is_empty() || !current_headers.is_empty() {
                    chunks.push((current_headers.clone(), current_content.clone()));
                    current_content.clear();
                }
                let header_text = trimmed
                    .trim_start_matches(&format!("{} ", level))
                    .trim()
                    .to_string();
                let normalized = normalize_level(&level);
                let depth = level_depth(&level);
                current_headers.retain(|k, _| key_depth(k) < depth);
                current_headers.insert(normalized, header_text);
            }
            None => {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }
    }

    if !current_content.is_empty() || !current_headers.is_empty() {
        chunks.push((current_headers, current_content));
    }

    chunks
}
