//! Stream types for compose module.

use tokio::sync::mpsc;
use std::marker::PhantomData;

/// Stream error type
pub type StreamError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Async stream reader for compose module.
pub struct StreamReader<T> {
    rx: mpsc::Receiver<Result<T, StreamError>>,
    _phantom: PhantomData<T>,
}

impl<T> StreamReader<T> {
    /// Create from a receiver.
    pub fn from_receiver(rx: mpsc::Receiver<Result<T, StreamError>>) -> Self {
        Self { rx, _phantom: PhantomData }
    }

    /// Receive the next item from the stream.
    pub async fn recv(&mut self) -> Option<Result<T, StreamError>> {
        self.rx.recv().await
    }

    /// Get the inner receiver.
    pub fn into_inner(self) -> mpsc::Receiver<Result<T, StreamError>> {
        self.rx
    }
}

impl<T: Clone + Send + 'static> StreamReader<T> {
    /// Convert this reader into a futures Stream.
    pub fn into_stream(self) -> impl futures::Stream<Item = Result<T, StreamError>> + Send {
        futures::stream::unfold(self.rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        })
    }
}

/// Async stream writer for compose module.
#[derive(Clone)]
pub struct StreamWriter<T> {
    tx: mpsc::Sender<Result<T, StreamError>>,
    _phantom: PhantomData<T>,
}

impl<T> StreamWriter<T> {
    /// Create from a sender.
    pub fn from_sender(tx: mpsc::Sender<Result<T, StreamError>>) -> Self {
        Self { tx, _phantom: PhantomData }
    }

    /// Send an item to the stream.
    pub async fn send(&mut self, item: Result<T, StreamError>) -> bool {
        self.tx.send(item).await.is_ok()
    }

    /// Close the channel.
    pub fn close(self) {
        drop(self.tx);
    }
}

/// Alias for channel.
pub fn pipe<T>(cap: usize) -> (StreamWriter<T>, StreamReader<T>) {
    channel(cap)
}

/// Create a channel pair for streaming.
pub fn channel<T>(cap: usize) -> (StreamWriter<T>, StreamReader<T>) {
    let (tx, rx) = mpsc::channel(cap);
    (StreamWriter::from_sender(tx), StreamReader::from_receiver(rx))
}

/// Create a stream reader that emits all items from an array and then closes.
pub fn from_array<T: Clone + Send + 'static>(arr: Vec<T>) -> StreamReader<T> {
    let cap = arr.len().max(1);
    let (mut w, r) = channel::<T>(cap);
    tokio::spawn(async move {
        for item in arr {
            let _ = w.send(Ok(item)).await;
        }
        drop(w);
    });
    r
}

/// Merge multiple stream readers round-robin.
pub async fn merge<T: Send + 'static>(mut readers: Vec<StreamReader<T>>) -> StreamReader<T> {
    let (mut w, r) = channel::<T>(32);
    tokio::spawn(async move {
        let mut offset: usize = 0;
        loop {
            let mut all_closed = true;
            let n = readers.len();
            for i in 0..n {
                let idx = (offset + i) % n;
                let r = &mut readers[idx];
                match r.recv().await {
                    Some(Ok(v)) => {
                        let _ = w.send(Ok(v)).await;
                        all_closed = false;
                    }
                    Some(Err(e)) => {
                        let _ = w.send(Err(e)).await;
                        all_closed = false;
                    }
                    None => {}
                }
            }
            if all_closed {
                break;
            }
            offset = offset.saturating_add(1);
        }
        drop(w);
    });
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_send_recv() {
        let (mut w, mut r) = channel::<String>(10);
        let _ = w.send(Ok("hello".to_string())).await;
        let result = r.recv().await;
        assert!(matches!(result, Some(Ok(s)) if s == "hello"));
    }

    #[tokio::test]
    async fn test_from_array() {
        let arr = vec![1, 2, 3];
        let mut r = from_array(arr);
        let v1 = r.recv().await;
        let v2 = r.recv().await;
        let v3 = r.recv().await;
        assert!(matches!(v1, Some(Ok(1))));
        assert!(matches!(v2, Some(Ok(2))));
        assert!(matches!(v3, Some(Ok(3))));
    }

    #[tokio::test]
    async fn test_merge() {
        let r1 = from_array(vec![1, 2]);
        let r2 = from_array(vec![3, 4]);
        let mut merged = merge(vec![r1, r2]).await;
        let mut results = Vec::new();
        while let Some(item) = merged.recv().await {
            results.push(item.unwrap());
        }
        assert_eq!(results.len(), 4);
    }
}
