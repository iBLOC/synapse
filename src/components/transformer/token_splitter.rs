
use serde_json::json;

use crate::components::document::DocumentResult;
use crate::components::retriever::Document;

use super::DocumentTransformer;
use async_trait::async_trait;

pub struct TokenTextSplitter {
    chunk_size: usize,
    chunk_overlap: usize,
}

impl TokenTextSplitter {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
        }
    }
}

#[async_trait]
impl DocumentTransformer for TokenTextSplitter {
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>> {
        let mut result = Vec::new();
        for doc in documents {
            let chunks = split_by_tokens(&doc.content, self.chunk_size, self.chunk_overlap);
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
        "TokenTextSplitter"
    }
}

fn approx_token_count(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    let char_count = text.len();
    std::cmp::max(word_count, char_count / 4)
}

fn split_by_tokens(text: &str, chunk_size: usize, chunk_overlap: usize) -> Vec<String> {
    if approx_token_count(text) <= chunk_size {
        return vec![text.to_string()];
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= chunk_size {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    let total_words = words.len();

    while start < total_words {
        let end = (start + chunk_size).min(total_words);
        let chunk: String = words[start..end].join(" ");
        chunks.push(chunk);

        if end == total_words {
            break;
        }
        start = end.saturating_sub(chunk_overlap);
    }

    chunks
}
