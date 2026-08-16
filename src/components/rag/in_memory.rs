use crate::components::embedding::Embedding;
use crate::components::indexer::{Indexer, IndexerError, IndexerResult};
use crate::components::retriever::{
    Document, RetrieveOptions, Retriever, RetrieverError, RetrieverResult,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct StoredDoc {
    doc: Document,
    embedding: Vec<f32>,
}

pub struct InMemoryVectorStore {
    docs: Arc<RwLock<Vec<StoredDoc>>>,
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self {
            docs: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryVectorStore {
    pub async fn add(&self, docs: Vec<Document>, embeddings: Vec<Vec<f32>>) {
        if docs.len() != embeddings.len() {
            return;
        }
        let mut store = self.docs.write().await;
        for (doc, embedding) in docs.into_iter().zip(embeddings.into_iter()) {
            store.push(StoredDoc { doc, embedding });
        }
    }

    pub async fn delete(&self, ids: Vec<String>) {
        let mut store = self.docs.write().await;
        store.retain(|d| !ids.contains(&d.doc.id));
    }

    pub async fn search(&self, query_emb: &[f32], top_k: usize) -> Vec<(f32, Document)> {
        let store = self.docs.read().await;
        let mut scored: Vec<(f32, &Document)> = store
            .iter()
            .map(|s| (cosine_similarity(query_emb, &s.embedding), &s.doc))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(top_k)
            .map(|(s, d)| (s, d.clone()))
            .collect()
    }

    pub async fn count(&self) -> usize {
        self.docs.read().await.len()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0_f32;
    let mut na = 0.0_f32;
    let mut nb = 0.0_f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

pub struct InMemoryIndexer {
    store: Arc<InMemoryVectorStore>,
    embedder: Arc<dyn Embedding>,
}

impl InMemoryIndexer {
    pub fn new(store: Arc<InMemoryVectorStore>, embedder: Arc<dyn Embedding>) -> Self {
        Self { store, embedder }
    }
}

#[async_trait::async_trait]
impl Indexer for InMemoryIndexer {
    async fn index(&self, documents: Vec<Document>) -> IndexerResult<()> {
        if documents.is_empty() {
            return Ok(());
        }
        let texts: Vec<String> = documents.iter().map(|d| d.content.clone()).collect();
        let embeddings = self
            .embedder
            .embed(texts)
            .await
            .map_err(|e| IndexerError::Index(format!("embedding failed: {}", e)))?;
        self.store.add(documents, embeddings).await;
        Ok(())
    }

    async fn index_from_text(&self, texts: Vec<String>) -> IndexerResult<()> {
        if texts.is_empty() {
            return Ok(());
        }
        let embeddings = self
            .embedder
            .embed(texts.clone())
            .await
            .map_err(|e| IndexerError::Index(format!("embedding failed: {}", e)))?;
        let documents: Vec<Document> = texts
            .into_iter()
            .enumerate()
            .map(|(i, t)| Document {
                id: format!("doc_{}", i),
                content: t,
                mime_type: "text/plain".to_string(),
                metadata: Default::default(),
            })
            .collect();
        self.store.add(documents, embeddings).await;
        Ok(())
    }

    async fn delete(&self, doc_ids: Vec<String>) -> IndexerResult<()> {
        self.store.delete(doc_ids).await;
        Ok(())
    }

    async fn flush(&self) -> IndexerResult<()> {
        Ok(())
    }
}

pub struct InMemoryRetriever {
    store: Arc<InMemoryVectorStore>,
    embedder: Arc<dyn Embedding>,
}

impl InMemoryRetriever {
    pub fn new(store: Arc<InMemoryVectorStore>, embedder: Arc<dyn Embedding>) -> Self {
        Self { store, embedder }
    }
}

#[async_trait::async_trait]
impl Retriever for InMemoryRetriever {
    async fn retrieve(
        &self,
        query: &str,
        options: RetrieveOptions,
    ) -> RetrieverResult<Vec<Document>> {
        let query_emb = self
            .embedder
            .embed(vec![query.to_string()])
            .await
            .map_err(|e| RetrieverError::Retrieve(format!("embedding failed: {}", e)))?
            .into_iter()
            .next()
            .ok_or_else(|| RetrieverError::InvalidQuery("empty embedding".to_string()))?;

        let top_k = options.top_k.unwrap_or(4);
        let threshold = options.threshold.unwrap_or(f32::NEG_INFINITY);

        let hits = self.store.search(&query_emb, top_k).await;
        Ok(hits
            .into_iter()
            .filter(|(score, _)| *score >= threshold)
            .map(|(_, doc)| doc)
            .collect())
    }
}
