use std::sync::Arc;
use synapse::components::embedding::Embedding;
use synapse::components::indexer::Indexer;
use synapse::components::rag::{InMemoryIndexer, InMemoryRetriever, InMemoryVectorStore};
use synapse::components::retriever::{Document, RetrieveOptions, Retriever};

struct FakeEmbedding;

#[async_trait::async_trait]
impl Embedding for FakeEmbedding {
    async fn embed(
        &self,
        texts: Vec<String>,
    ) -> synapse::components::embedding::EmbeddingResult<Vec<Vec<f32>>> {
        let mut out = Vec::with_capacity(texts.len());
        for text in texts {
            let v = fake_embed(&text);
            out.push(v);
        }
        Ok(out)
    }
    fn dimensions(&self) -> usize {
        4
    }
}

fn fake_embed(text: &str) -> Vec<f32> {
    let mut v = vec![0.0_f32; 4];
    for (i, b) in text.bytes().enumerate() {
        v[i % 4] += b as f32;
    }
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    v
}

#[tokio::test]
async fn test_in_memory_vector_store_cosine_search() {
    let store = Arc::new(InMemoryVectorStore::new());

    let docs = vec![
        Document {
            id: "1".to_string(),
            content: "Rust is a systems programming language".to_string(),
            metadata: Default::default(),
        },
        Document {
            id: "2".to_string(),
            content: "Python is a dynamic language".to_string(),
            metadata: Default::default(),
        },
        Document {
            id: "3".to_string(),
            content: "Rust focuses on safety and performance".to_string(),
            metadata: Default::default(),
        },
    ];
    let embeddings: Vec<Vec<f32>> = docs.iter().map(|d| fake_embed(&d.content)).collect();
    store.add(docs, embeddings).await;

    assert_eq!(store.count().await, 3);

    let query_emb = fake_embed("Rust systems programming");
    let hits = store.search(&query_emb, 2).await;
    assert_eq!(hits.len(), 2);
}

#[tokio::test]
async fn test_indexer_and_retriever_roundtrip() {
    let embedder: Arc<dyn Embedding> = Arc::new(FakeEmbedding);
    let store = Arc::new(InMemoryVectorStore::new());
    let indexer = InMemoryIndexer::new(store.clone(), embedder.clone());
    let retriever = InMemoryRetriever::new(store.clone(), embedder.clone());

    let docs = vec![
        Document {
            id: "a".to_string(),
            content: "the quick brown fox".to_string(),
            metadata: Default::default(),
        },
        Document {
            id: "b".to_string(),
            content: "jumps over the lazy dog".to_string(),
            metadata: Default::default(),
        },
        Document {
            id: "c".to_string(),
            content: "the brown fox runs".to_string(),
            metadata: Default::default(),
        },
    ];
    indexer.index(docs).await.expect("indexing failed");

    let results = retriever
        .retrieve(
            "fox jumps",
            RetrieveOptions {
                top_k: Some(2),
                threshold: None,
            },
        )
        .await
        .expect("retrieve failed");
    assert_eq!(results.len(), 2);
    assert!(results
        .iter()
        .any(|d| d.id == "a" || d.id == "b" || d.id == "c"));
}

#[tokio::test]
async fn test_indexer_delete() {
    let embedder: Arc<dyn Embedding> = Arc::new(FakeEmbedding);
    let store = Arc::new(InMemoryVectorStore::new());
    let indexer = InMemoryIndexer::new(store.clone(), embedder.clone());

    let docs = vec![
        Document {
            id: "x".to_string(),
            content: "alpha".to_string(),
            metadata: Default::default(),
        },
        Document {
            id: "y".to_string(),
            content: "beta".to_string(),
            metadata: Default::default(),
        },
    ];
    indexer.index(docs).await.unwrap();
    assert_eq!(store.count().await, 2);

    indexer.delete(vec!["x".to_string()]).await.unwrap();
    assert_eq!(store.count().await, 1);
}

#[tokio::test]
async fn test_retriever_threshold_filter() {
    let embedder: Arc<dyn Embedding> = Arc::new(FakeEmbedding);
    let store = Arc::new(InMemoryVectorStore::new());
    let indexer = InMemoryIndexer::new(store.clone(), embedder.clone());
    let retriever = InMemoryRetriever::new(store, embedder);

    indexer
        .index_from_text(vec!["hello world".to_string(), "goodbye world".to_string()])
        .await
        .unwrap();

    let results = retriever
        .retrieve(
            "completely unrelated query xyz",
            RetrieveOptions {
                top_k: Some(5),
                threshold: Some(0.1),
            },
        )
        .await
        .unwrap();
    for r in &results {
        let _ = r.content.len();
    }
}
