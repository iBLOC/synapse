//! Unified, cancellation-aware streaming primitives for Synapse.
//!
//! This module provides a single, generic `StreamReader<T>` that replaces the
//! two parallel stream types previously living in `components::model` and
//! `compose::runnable`. The unified type:
//!
//! - carries **raw values** of type `T` (not `Result<T, E>`); errors are
//!   modeled by the value type itself, e.g. [`crate::adk::stream::AgentStreamEvent::Error`]
//! - exposes an `Arc<AtomicBool>`-backed [`CancelHandle`] that any number of
//!   producers or consumers may clone
//! - implements [`futures::Stream`] for use with the wider async ecosystem
//!
//! Backwards compatibility: the concrete (model-layer) and the
//! generic-but-error-bearing (compose-layer) stream types are preserved and
//! re-exported from their original locations. See [`compat`] below.

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::{Stream, StreamExt};
use tokio::sync::mpsc;

/// The consumer side of a unified Synapse stream.
///
/// Constructed via [`StreamReader::new`] (paired with a [`StreamWriter`]) or
/// via [`StreamReader::from_receiver`] for hand-rolled mpsc channels.
pub struct StreamReader<T> {
    rx: mpsc::Receiver<T>,
    cancel: Arc<AtomicBool>,
}

/// The producer side of a unified Synapse stream.
///
/// `StreamWriter` is `Clone` so a single producer may hand the writer to
/// several sub-tasks. Dropping the last `StreamWriter` causes the channel to
/// close and the corresponding `StreamReader` to start yielding `None`.
#[derive(Clone)]
pub struct StreamWriter<T> {
    tx: mpsc::Sender<T>,
}

/// A cheap, cloneable handle to a stream's cancellation flag.
///
/// Cloning a `CancelHandle` shares the same `Arc<AtomicBool>` as the source
/// `StreamReader` and as every other clone. Calling [`CancelHandle::cancel`]
/// signals every observer; producers should poll [`CancelHandle::is_cancelled`]
/// at every await point and bail out early when it returns `true`.
#[derive(Clone)]
pub struct CancelHandle {
    flag: Arc<AtomicBool>,
}

impl CancelHandle {
    /// Signal cancellation to every observer of this stream.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Returns `true` once [`Self::cancel`] has been called on **any** clone.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

impl<T> StreamReader<T> {
    /// Create a paired `(StreamWriter, StreamReader)` with a bounded channel
    /// of capacity `cap`. The cancel flag is fresh (unset).
    pub fn new(cap: usize) -> (StreamWriter<T>, Self) {
        let (tx, rx) = mpsc::channel(cap);
        let cancel = Arc::new(AtomicBool::new(false));
        (
            StreamWriter { tx },
            StreamReader { rx, cancel },
        )
    }

    /// Build a reader from an existing `mpsc::Receiver`, with a fresh cancel
    /// flag. Useful for tests and for adapting hand-rolled channels.
    pub fn from_receiver(rx: mpsc::Receiver<T>) -> Self {
        Self {
            rx,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Borrow a clone of the cancellation handle. The flag starts unset;
    /// call [`CancelHandle::cancel`] to abort producers.
    pub fn cancel_handle(&self) -> CancelHandle {
        CancelHandle {
            flag: self.cancel.clone(),
        }
    }

    /// Returns `true` if any handle observing this stream has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// Convert into a boxed `Stream` for use with `futures` combinators.
    pub fn into_stream(self) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
        T: Send + 'static,
    {
        Box::pin(self)
    }

    /// Receive the next value, awaiting if the channel is empty. Returns
    /// `None` when the last `StreamWriter` has been dropped.
    pub async fn next(&mut self) -> Option<T> {
        self.rx.recv().await
    }
}

impl<T: Send + 'static> Stream for StreamReader<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_recv(cx)
    }
}

impl<T> StreamWriter<T> {
    /// Send a value, returning `false` if the consumer has been dropped.
    pub async fn send(&self, value: T) -> bool {
        self.tx.send(value).await.is_ok()
    }

    /// Convenience alias for [`Self::send`].
    pub async fn send_value(&self, value: T) -> bool {
        self.send(value).await
    }

    /// Returns `true` when the consumer side has been dropped and the channel
    /// is therefore closed.
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }

    /// Returns the current capacity of the underlying bounded channel.
    pub fn capacity(&self) -> usize {
        self.tx.capacity()
    }
}

/// Backwards-compatibility shims. The pre-1.0 stream types still exist; this
/// module re-exports adapters so legacy call sites keep working while the rest
/// of the codebase migrates to the unified [`StreamReader`].
pub mod compat {
    use super::*;

    /// Adapt a `StreamReader<T>` (raw values) into a stream of
    /// `Result<T, ()>`. Used by code that was written against the legacy
    /// `StreamReader<Result<T, _>>` shape and has not yet been ported.
    pub fn results_only<T: Send + 'static>(
        reader: StreamReader<T>,
    ) -> impl futures::Stream<Item = Result<T, ()>> + Send {
        reader.map(|v| Ok(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn stream_reader_send_recv() {
        let (writer, mut reader) = StreamReader::<u32>::new(4);
        assert!(writer.send(1).await);
        assert!(writer.send(2).await);
        assert_eq!(reader.next().await, Some(1));
        assert_eq!(reader.next().await, Some(2));
        // Dropping the writer closes the channel.
        drop(writer);
        assert_eq!(reader.next().await, None);
    }

    #[tokio::test]
    async fn stream_reader_cancel_propagates() {
        let (writer, reader) = StreamReader::<u32>::new(4);
        let handle = reader.cancel_handle();
        assert!(!reader.is_cancelled());
        handle.cancel();
        assert!(reader.is_cancelled());
        // A cloned handle sees the same flag.
        let h2 = reader.cancel_handle();
        assert!(h2.is_cancelled());
        // Writer still works; the cancel flag is a *signal*, not a channel close.
        assert!(writer.send(42).await);
    }

    #[tokio::test]
    async fn stream_writer_clone_works() {
        let (writer, mut reader) = StreamReader::<&'static str>::new(4);
        let writer2 = writer.clone();
        assert!(writer.send("a").await);
        assert!(writer2.send("b").await);
        let a = reader.next().await;
        let b = reader.next().await;
        assert!(matches!(a, Some("a")));
        assert!(matches!(b, Some("b")));
    }

    #[tokio::test]
    async fn stream_reader_into_stream() {
        let (writer, reader) = StreamReader::<u32>::new(4);
        tokio::spawn(async move {
            assert!(writer.send(1).await);
            assert!(writer.send(2).await);
            assert!(writer.send(3).await);
        });
        let s = reader.into_stream();
        let collected: Vec<u32> = s.collect().await;
        assert_eq!(collected, vec![1, 2, 3]);
    }
}
