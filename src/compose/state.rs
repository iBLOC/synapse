//! Graph state management with mutex-based concurrency safety.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

pub type StateResult<T> = Result<T, StateError>;

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("state not found")]
    NotFound,
    #[error("state type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    #[error("state error: {0}")]
    Other(String),
}

struct InternalState {
    state: TokioMutex<serde_json::Value>,
    parent: Option<Arc<InternalState>>,
}

// Manual Debug implementation since TokioMutex doesn't implement Debug
impl std::fmt::Debug for InternalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InternalState")
            .field("parent", &self.parent.is_some())
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct State<S: Clone + Send + Sync + 'static + serde::Serialize + serde::de::DeserializeOwned>
{
    inner: Arc<InternalState>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Clone + Send + Sync + 'static + serde::Serialize + serde::de::DeserializeOwned> State<S> {
    pub fn new(state: S) -> Self {
        let json = serde_json::to_value(&state).unwrap_or(serde_json::Value::Null);
        Self {
            inner: Arc::new(InternalState {
                state: TokioMutex::new(json),
                parent: None,
            }),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_parent(parent: State<S>) -> Self {
        Self {
            inner: Arc::new(InternalState {
                state: TokioMutex::new(serde_json::Value::Null),
                parent: Some(parent.inner.clone()),
            }),
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn get(&self) -> StateResult<S> {
        let state = self.inner.state.lock().await.clone();
        serde_json::from_value(state).map_err(|e| StateError::Other(e.to_string()))
    }

    pub async fn set(&self, value: S) -> StateResult<()> {
        let json = serde_json::to_value(&value).map_err(|e| StateError::Other(e.to_string()))?;
        *self.inner.state.lock().await = json;
        Ok(())
    }

    pub async fn update<F, Fut>(&self, f: F) -> StateResult<S>
    where
        F: FnOnce(S) -> Fut,
        Fut: std::future::Future<Output = S>,
    {
        let mut guard = self.inner.state.lock().await;
        let current: S =
            serde_json::from_value(guard.clone()).map_err(|e| StateError::Other(e.to_string()))?;
        let new_state = f(current).await;
        *guard = serde_json::to_value(&new_state).map_err(|e| StateError::Other(e.to_string()))?;
        Ok(new_state)
    }

    pub async fn get_with_parent(&self) -> StateResult<S> {
        if let Some(ref parent) = self.inner.parent {
            let self_state: Result<S, _> =
                serde_json::from_value(self.inner.state.lock().await.clone());
            if self_state.is_err() || self.inner.state.lock().await.is_null() {
                let parent_json = parent.state.lock().await.clone();
                return serde_json::from_value(parent_json)
                    .map_err(|e| StateError::Other(e.to_string()));
            }
        }
        self.get().await
    }
}

pub struct StateManager {
    states: parking_lot::RwLock<HashMap<std::any::TypeId, Arc<dyn std::any::Any + Send + Sync>>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            states: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    pub fn register<
        S: Clone + Send + Sync + 'static + serde::Serialize + serde::de::DeserializeOwned,
    >(
        &self,
        initial: S,
    ) {
        let state = State::new(initial);
        let any: Arc<dyn std::any::Any + Send + Sync> = state.inner.clone();
        self.states.write().insert(std::any::TypeId::of::<S>(), any);
    }

    pub fn get<
        S: Clone + Send + Sync + 'static + serde::Serialize + serde::de::DeserializeOwned,
    >(
        &self,
    ) -> Option<State<S>> {
        let states = self.states.read();
        states.get(&std::any::TypeId::of::<S>()).map(|any| {
            let inner: Arc<InternalState> =
                any.downcast_ref::<Arc<InternalState>>().unwrap().clone();
            State {
                inner,
                _phantom: std::marker::PhantomData,
            }
        })
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct TestState {
        counter: i32,
        name: String,
    }

    #[tokio::test]
    async fn test_state_basic() {
        let state = State::new(TestState {
            counter: 0,
            name: "test".to_string(),
        });

        let retrieved = state.get().await.unwrap();
        assert_eq!(retrieved.counter, 0);

        state
            .set(TestState {
                counter: 10,
                name: "updated".to_string(),
            })
            .await
            .unwrap();

        let updated = state.get().await.unwrap();
        assert_eq!(updated.counter, 10);
    }

    #[tokio::test]
    async fn test_state_update() {
        let state = State::new(0i32);

        let result = state.update(|n| async move { n + 1 }).await.unwrap();
        assert_eq!(result, 1);

        let current = state.get().await.unwrap();
        assert_eq!(current, 1);
    }

    #[tokio::test]
    async fn test_state_basic_operations() {
        let state = State::new(42i32);
        assert_eq!(state.get().await.unwrap(), 42);
        state.set(100).await.unwrap();
        assert_eq!(state.get().await.unwrap(), 100);
    }
}
