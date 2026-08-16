//! Stream-forwarding helpers for agent composition.

use crate::stream::StreamWriter;
use futures::Stream;
use futures::StreamExt;

/// Forward every item from `inner` into `writer` until the inner stream is
/// exhausted or the writer's channel is closed.
///
/// Returns `true` if the inner stream ended naturally; `false` if the
/// writer was closed (consumer dropped) and the forward was aborted early.
pub async fn forward_to<T>(mut inner: impl Stream<Item = T> + Unpin, writer: &StreamWriter<T>) -> bool {
    while let Some(item) = inner.next().await {
        if !writer.send(item).await {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::StreamReader;
    use futures::stream;

    #[tokio::test]
    async fn forward_to_end_to_end() {
        let (writer, mut reader) = StreamReader::<u32>::new(8);
        let src = stream::iter(vec![1u32, 2, 3]);
        let completed = forward_to(src, &writer).await;
        assert!(completed);
        drop(writer);
        let v1 = reader.next().await;
        let v2 = reader.next().await;
        let v3 = reader.next().await;
        let v4 = reader.next().await;
        assert_eq!(v1, Some(1));
        assert_eq!(v2, Some(2));
        assert_eq!(v3, Some(3));
        assert_eq!(v4, None);
    }

    #[tokio::test]
    async fn forward_to_aborts_when_writer_closed() {
        let (writer, reader) = StreamReader::<u32>::new(8);
        drop(reader); // consumer side closed before producer starts
        let src = stream::iter(vec![1u32, 2, 3]);
        let completed = forward_to(src, &writer).await;
        assert!(!completed);
    }
}
