#[cfg(test)]
mod tests {
    use crate::compose::runnable::{channel, pipe, from_array, merge};

    #[tokio::test]
    async fn test_channel_basic() {
        let (mut tx, mut rx) = channel::<i32>(10);
        
        // Send values
        assert!(tx.send(Ok(1)).await);
        assert!(tx.send(Ok(2)).await);
        assert!(tx.send(Ok(3)).await);
        
        // Close the sender
        tx.close().await;
        
        // Receive values
        assert_eq!(rx.recv().await.unwrap().unwrap(), 1);
        assert_eq!(rx.recv().await.unwrap().unwrap(), 2);
        assert_eq!(rx.recv().await.unwrap().unwrap(), 3);
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn test_channel_error_propagation() {
        let (mut tx, mut rx) = channel::<i32>(10);
        
        // Send an error
        let err = Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
        assert!(tx.send(Err(err)).await);
        
        tx.close().await;
        
        // Receive the error
        let result = rx.recv().await;
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[tokio::test]
    async fn test_pipe_basic() {
        let (mut tx, mut rx) = pipe::<String>(5);
        
        tx.send(Ok("hello".to_string())).await;
        tx.send(Ok("world".to_string())).await;
        tx.close().await;
        
        assert_eq!(rx.recv().await.unwrap().unwrap(), "hello");
        assert_eq!(rx.recv().await.unwrap().unwrap(), "world");
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn test_from_array() {
        let arr = vec![1, 2, 3, 4, 5];
        let mut rx = from_array(arr);
        
        assert_eq!(rx.recv().await.unwrap().unwrap(), 1);
        assert_eq!(rx.recv().await.unwrap().unwrap(), 2);
        assert_eq!(rx.recv().await.unwrap().unwrap(), 3);
        assert_eq!(rx.recv().await.unwrap().unwrap(), 4);
        assert_eq!(rx.recv().await.unwrap().unwrap(), 5);
        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn test_merge() {
        let (mut tx1, rx1) = channel::<i32>(5);
        let (mut tx2, rx2) = channel::<i32>(5);
        
        // Create readers
        let mut readers = vec![rx1, rx2];
        
        // Send to first reader
        tx1.send(Ok(1)).await;
        tx1.send(Ok(2)).await;
        tx1.close().await;
        
        // Send to second reader
        tx2.send(Ok(3)).await;
        tx2.send(Ok(4)).await;
        tx2.close().await;
        
        // Merge the readers
        let mut merged = merge(readers).await;
        
        // Collect all results
        let mut results = Vec::new();
        while let Some(result) = merged.recv().await {
            results.push(result);
        }
        
        // Should have all 4 values
        assert_eq!(results.len(), 4);
    }
}
