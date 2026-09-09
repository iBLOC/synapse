//! ZitiStream connection pool — reuse `ZitiStream`s per service name
//! to avoid the cost of repeated `dial` for short-lived calls.
//!
//! Streams returned to the pool are kept until `max_connections`;
//! callers can ignore the pool entirely if they want strict
//! one-stream-per-call.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use ziti_sdk::ZitiStream;

use super::gateway::ZitiGateway;

pub struct ConnectionPool {
    gateway: Arc<ZitiGateway>,
    connections: Arc<Mutex<HashMap<String, Vec<ZitiStream>>>>,
    max_connections: usize,
}

impl ConnectionPool {
    pub fn new(gateway: Arc<ZitiGateway>, max_connections: usize) -> Self {
        Self {
            gateway,
            connections: Arc::new(Mutex::new(HashMap::new())),
            max_connections,
        }
    }

    pub async fn get_connection(&self, service_name: &str) -> Result<ZitiStream> {
        let mut connections = self.connections.lock().await;

        if let Some(pool) = connections.get_mut(service_name) {
            if let Some(conn) = pool.pop() {
                return Ok(conn);
            }
        }

        let stream = self.gateway.dial(service_name).await?;
        Ok(stream)
    }

    pub async fn return_connection(&self, service_name: &str, conn: ZitiStream) {
        let mut connections = self.connections.lock().await;

        let pool = connections
            .entry(service_name.to_string())
            .or_insert_with(Vec::new);

        if pool.len() < self.max_connections {
            pool.push(conn);
        }
    }

    pub async fn clear(&self) {
        let mut connections = self.connections.lock().await;
        connections.clear();
    }

    pub async fn stats(&self) -> HashMap<String, usize> {
        let connections = self.connections.lock().await;
        connections.iter().map(|(k, v)| (k.clone(), v.len())).collect()
    }
}