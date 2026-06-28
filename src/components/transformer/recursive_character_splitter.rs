use async_trait::async_trait;
use serde_json::json;

use crate::components::document::DocumentResult;
use crate::components::retriever::Document;

use super::DocumentTransformer;

pub struct RecursiveCharacterTextSplitter {
    chunk_size: usize,
    chunk_overlap: usize,
    separators: Vec<String>,
}

impl RecursiveCharacterTextSplitter {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
            separators: vec!["\n\n".into(), "\n".into(), " ".into(), "".into()],
        }
    }
}

#[async_trait]
impl DocumentTransformer for RecursiveCharacterTextSplitter {
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>> {
        let mut result = Vec::new();
        for doc in documents {
            let chunks = recursive_split(
                &doc.content,
                self.chunk_size,
                self.chunk_overlap,
                &self.separators,
            );
            for (i, chunk_text) in chunks.into_iter().enumerate() {
                let mut metadata = doc.metadata.clone();
                metadata.insert("source_id".into(), json!(doc.id));
                metadata.insert("chunk_index".into(), json!(i));
                result.push(Document {
                    id: format!("{}_{}", doc.id, i),
                    content: chunk_text,
                    mime_type: doc.mime_type.clone(),
                    metadata,
                });
            }
        }
        Ok(result)
    }

    fn name(&self) -> &str {
        "RecursiveCharacterTextSplitter"
    }
}

fn recursive_split(
    text: &str,
    chunk_size: usize,
    _chunk_overlap: usize,
    separators: &[String],
) -> Vec<String> {
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }

    let sep = &separators[0];
    let remaining = &separators[1..];

    if sep.is_empty() {
        return hard_split(text, chunk_size);
    }

    let pieces: Vec<String> = text
        .split(sep)
        .enumerate()
        .map(|(i, p)| {
            if i == 0 {
                p.to_string()
            } else {
                format!("{}{}", sep, p)
            }
        })
        .collect();

    let mut chunks = Vec::new();
    let mut current = String::new();

    for piece in pieces {
        if current.len() + piece.len() > chunk_size && !current.is_empty() {
            chunks.push(current);
            current = piece;
        } else {
            current.push_str(&piece);
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
        .into_iter()
        .flat_map(|c| {
            if c.len() > chunk_size && !remaining.is_empty() {
                recursive_split(&c, chunk_size, _chunk_overlap, remaining)
            } else if c.len() > chunk_size {
                hard_split(&c, chunk_size)
            } else {
                vec![c]
            }
        })
        .collect()
}

fn hard_split(text: &str, chunk_size: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(chunk_size)
        .map(|c| c.iter().collect())
        .collect()
}
