use tokio::sync::mpsc;

pub type StreamError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct StreamReader<T> {
    rx: mpsc::Receiver<Result<T, StreamError>>,
}

impl<T> StreamReader<T> {
    pub fn from_receiver(rx: mpsc::Receiver<Result<T, StreamError>>) -> Self {
        Self { rx }
    }

    pub async fn recv(&mut self) -> Option<Result<T, StreamError>> {
        self.rx.recv().await
    }
}

pub struct StreamWriter<T> {
    tx: mpsc::Sender<Result<T, StreamError>>,
}

impl<T> StreamWriter<T> {
    pub fn from_sender(tx: mpsc::Sender<Result<T, StreamError>>) -> Self {
        Self { tx }
    }

    pub async fn send(&mut self, item: Result<T, StreamError>) -> bool {
        self.tx.send(item).await.is_ok()
    }

    pub async fn close(self) {
        // Drop the sender to signal channel closure
        drop(self.tx);
    }
}

pub fn channel<T>(cap: usize) -> (StreamWriter<T>, StreamReader<T>) {
    let (tx, rx) = mpsc::channel(cap);
    (StreamWriter::from_sender(tx), StreamReader::from_receiver(rx))
}

pub fn pipe<T: Clone + Send + 'static>(cap: usize) -> (StreamWriter<T>, StreamReader<T>) {
    let (tx, rx) = mpsc::channel(cap);
    (StreamWriter::from_sender(tx), StreamReader::from_receiver(rx))
}

pub fn from_array<T: Clone + Send + 'static>(arr: Vec<T>) -> StreamReader<T> {
    let (tx, rx) = mpsc::channel(arr.len());
    let arr_clone = arr.clone();
    
    tokio::spawn(async move {
        for item in arr_clone {
            if tx.send(Ok(item)).await.is_err() {
                break;
            }
        }
    });
    
    StreamReader::from_receiver(rx)
}

pub async fn merge<T: Send + 'static>(mut readers: Vec<StreamReader<T>>) -> StreamReader<T> {
    let (tx, rx) = mpsc::channel::<Result<T, StreamError>>(100);
    
    tokio::spawn(async move {
        loop {
            let mut all_closed = true;
            
            for reader in readers.iter_mut() {
                if let Some(result) = reader.recv().await {
                    all_closed = false;
                    if tx.send(result).await.is_err() {
                        return;
                    }
                }
            }
            
            if all_closed {
                break;
            }
        }
    });
    
    StreamReader::from_receiver(rx)
}
