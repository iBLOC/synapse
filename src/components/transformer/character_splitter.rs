use async_trait::async_trait;
use serde_json::json;

use crate::components::document::DocumentResult;
use crate::components::retriever::Document;

use super::DocumentTransformer;

pub struct CharacterTextSplitter {
    chunk_size: usize,
    chunk_overlap: usize,
    separator: String,
}

impl CharacterTextSplitter {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
            separator: "\n".to_string(),
        }
    }
}

#[async_trait]
impl DocumentTransformer for CharacterTextSplitter {
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>> {
        let mut result = Vec::new();
        for doc in documents {
            let chunks = split_by_char(
                &doc.content,
                self.chunk_size,
                self.chunk_overlap,
                &self.separator,
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
        "CharacterTextSplitter"
    }
}

fn split_by_char(
    text: &str,
    chunk_size: usize,
    chunk_overlap: usize,
    sep: &str,
) -> Vec<String> {
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let char_len = chars.len();
    let mut start = 0;

    while start < char_len {
        let end = (start + chunk_size).min(char_len);
        let mut chunk: String = chars[start..end].iter().collect();

        if end < char_len {
            if let Some(last_sep_pos) = chunk.rfind(sep) {
                if last_sep_pos > chunk_size / 2 {
                    chunk.truncate(last_sep_pos + sep.len());
                }
            }
        }

        chunks.push(chunk);

        if end == char_len {
            break;
        }
        start = end.saturating_sub(chunk_overlap);
    }

    chunks
}
