//! # WEBSOCKET CONNECTION IMPLEMENTATION
//!
//! **HIGH-PERFORMANCE CONNECTION MANAGEMENT WITH BACKPRESSURE**
//!
//! This module implements WebSocket connections with efficient message queuing,
//! automatic ping/pong handling, and backpressure management.

use super::{Message, CloseCode, WebSocketError, WebSocketConfig, ConnectionMetrics};
use bytes::Bytes;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time::{interval, timeout};
use uuid::Uuid;

/// **CONNECTION STATE**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection is being established
    Connecting,
    /// Connection is open and active
    Open,
    /// Connection is closing
    Closing,
    /// Connection is closed
    Closed,
}

/// **WEBSOCKET CONNECTION**
///
/// **PURPOSE**: Represent a single WebSocket connection with efficient message handling.
/// **GUARANTEE**: Thread-safe, backpressure-aware message processing.
pub struct WebSocketConnection {
    /// Unique connection ID
    id: Uuid,
    
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    
    /// Send channel
    tx: mpsc::Sender<Message>,
    
    /// Receive channel
    rx: Arc<Mutex<mpsc::Receiver<Message>>>,
    
    /// Last ping time
    last_ping: Arc<RwLock<Instant>>,
    
    /// Last pong time
    last_pong: Arc<RwLock<Instant>>,
    
    /// Connection metrics
    metrics: Arc<RwLock<ConnectionMetrics>>,
    
    /// Configuration
    config: Arc<WebSocketConfig>,
    
    /// Connection metadata
    metadata: Arc<RwLock<anymap::AnyMap>>,
}

impl WebSocketConnection {
    /// **CREATE NEW CONNECTION**
    pub fn new(config: Arc<WebSocketConfig>) -> Self {
        let (tx, rx) = mpsc::channel(config.send_buffer_size);
        let now = Instant::now();
        
        Self {
            id: Uuid::new_v4(),
            state: Arc::new(RwLock::new(ConnectionState::Connecting)),
            tx,
            rx: Arc::new(Mutex::new(rx)),
            last_ping: Arc::new(RwLock::new(now)),
            last_pong: Arc::new(RwLock::new(now)),
            metrics: Arc::new(RwLock::new(ConnectionMetrics::default())),
            config,
            metadata: Arc::new(RwLock::new(anymap::AnyMap::new())),
        }
    }
    
    /// **GET CONNECTION ID**
    pub fn id(&self) -> Uuid {
        self.id
    }
    
    /// **GET CONNECTION STATE**
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }
    
    /// **SET CONNECTION STATE**
    pub async fn set_state(&self, state: ConnectionState) {
        *self.state.write().await = state;
    }
    
    /// **SEND MESSAGE**
    ///
    /// **BACKPRESSURE**: Returns error if send buffer is full
    pub async fn send(&self, message: Message) -> Result<(), WebSocketError> {
        // Check state
        let state = self.state().await;
        if state != ConnectionState::Open {
            return Err(WebSocketError::ConnectionClosed(
                format!("Connection in state {:?}", state)
            ));
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.messages_sent += 1;
            metrics.bytes_sent += message.len() as u64;
        }
        
        // Send with timeout to prevent blocking
        match timeout(
            Duration::from_secs(5),
            self.tx.send(message)
        ).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(WebSocketError::ConnectionClosed("Send channel closed".to_string())),
            Err(_) => Err(WebSocketError::ConnectionClosed("Send timeout".to_string())),
        }
    }
    
    /// **RECEIVE MESSAGE**
    pub async fn receive(&self) -> Result<Option<Message>, WebSocketError> {
        // Check state
        let state = self.state().await;
        if state == ConnectionState::Closed {
            return Ok(None);
        }
        
        // Receive from channel
        let mut rx = self.rx.lock().await;
        match rx.recv().await {
            Some(message) => {
                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.messages_received += 1;
                metrics.bytes_received += message.len() as u64;
                
                Ok(Some(message))
            }
            None => Ok(None),
        }
    }
    
    /// **CLOSE CONNECTION**
    pub async fn close(&self, code: CloseCode, reason: &str) -> Result<(), WebSocketError> {
        // Update state
        self.set_state(ConnectionState::Closing).await;
        
        // Send close message
        let close_msg = Message::close(code, reason);
        let _ = self.send(close_msg).await;
        
        // Update state
        self.set_state(ConnectionState::Closed).await;
        
        Ok(())
    }
    
    /// **START PING TASK**
    ///
    /// **PURPOSE**: Automatically send ping messages to keep connection alive
    pub async fn start_ping_task(self: Arc<Self>) {
        let ping_interval = Duration::from_secs(self.config.ping_interval);
        let mut interval = interval(ping_interval);
        
        loop {
            interval.tick().await;
            
            // Check if connection is still open
            if self.state().await != ConnectionState::Open {
                break;
            }
            
            // Send ping
            let ping_data = Bytes::from(Uuid::new_v4().as_bytes().to_vec());
            if let Err(e) = self.send(Message::ping(ping_data)).await {
                log::warn!("Failed to send ping: {}", e);
                break;
            }
            
            // Update last ping time
            *self.last_ping.write().await = Instant::now();
            
            // Check for pong timeout
            let last_pong = *self.last_pong.read().await;
            if last_pong.elapsed() > Duration::from_secs(self.config.pong_timeout) {
                log::warn!("Pong timeout for connection {}", self.id);
                let _ = self.close(CloseCode::Away, "Pong timeout").await;
                break;
            }
        }
    }
    
    /// **HANDLE PONG**
    pub async fn handle_pong(&self, _data: Bytes) {
        *self.last_pong.write().await = Instant::now();
    }
    
    /// **GET METRICS**
    pub async fn metrics(&self) -> ConnectionMetrics {
        self.metrics.read().await.clone()
    }
    
    /// **SET METADATA**
    ///
    /// Store arbitrary metadata with the connection
    pub async fn set_metadata<T: Send + Sync + 'static>(&self, value: T) {
        self.metadata.write().await.insert(value);
    }
    
    /// **GET METADATA**
    pub async fn get_metadata<T: Send + Sync + 'static + Clone>(&self) -> Option<T> {
        self.metadata.read().await.get::<T>().cloned()
    }
    
    /// **CHECK IF CONNECTED**
    pub async fn is_connected(&self) -> bool {
        self.state().await == ConnectionState::Open
    }
    
    /// **GET SEND QUEUE SIZE**
    pub fn send_queue_size(&self) -> usize {
        self.config.send_buffer_size - self.tx.capacity()
    }
}

/// **CONNECTION POOL**
///
/// **PURPOSE**: Efficiently manage multiple connections
pub struct ConnectionPool {
    /// Active connections
    connections: Arc<RwLock<HashMap<Uuid, Arc<WebSocketConnection>>>>,
    
    /// Maximum connections
    max_connections: usize,
}

impl ConnectionPool {
    /// **CREATE NEW POOL**
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
        }
    }
    
    /// **ADD CONNECTION**
    pub async fn add(&self, connection: Arc<WebSocketConnection>) -> Result<(), WebSocketError> {
        let mut connections = self.connections.write().await;
        
        if connections.len() >= self.max_connections {
            return Err(WebSocketError::ConnectionClosed(
                "Connection pool full".to_string()
            ));
        }
        
        connections.insert(connection.id(), connection);
        Ok(())
    }
    
    /// **REMOVE CONNECTION**
    pub async fn remove(&self, id: Uuid) -> Option<Arc<WebSocketConnection>> {
        self.connections.write().await.remove(&id)
    }
    
    /// **GET CONNECTION**
    pub async fn get(&self, id: Uuid) -> Option<Arc<WebSocketConnection>> {
        self.connections.read().await.get(&id).cloned()
    }
    
    /// **GET ALL CONNECTIONS**
    pub async fn all(&self) -> Vec<Arc<WebSocketConnection>> {
        self.connections.read().await.values().cloned().collect()
    }
    
    /// **CONNECTION COUNT**
    pub async fn count(&self) -> usize {
        self.connections.read().await.len()
    }
    
    /// **BROADCAST TO ALL**
    pub async fn broadcast(&self, message: Message) -> Result<usize, WebSocketError> {
        let connections = self.all().await;
        let mut sent = 0;
        
        for conn in connections {
            if conn.is_connected().await {
                if conn.send(message.clone()).await.is_ok() {
                    sent += 1;
                }
            }
        }
        
        Ok(sent)
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection_creation() {
        let config = Arc::new(WebSocketConfig::default());
        let conn = WebSocketConnection::new(config);
        
        assert_eq!(conn.state().await, ConnectionState::Connecting);
        assert_eq!(conn.send_queue_size(), 0);
    }
    
    #[tokio::test]
    async fn test_connection_state_transitions() {
        let config = Arc::new(WebSocketConfig::default());
        let conn = WebSocketConnection::new(config);
        
        // Connecting -> Open
        conn.set_state(ConnectionState::Open).await;
        assert_eq!(conn.state().await, ConnectionState::Open);
        assert!(conn.is_connected().await);
        
        // Open -> Closing
        conn.set_state(ConnectionState::Closing).await;
        assert_eq!(conn.state().await, ConnectionState::Closing);
        assert!(!conn.is_connected().await);
        
        // Closing -> Closed
        conn.set_state(ConnectionState::Closed).await;
        assert_eq!(conn.state().await, ConnectionState::Closed);
    }
    
    #[tokio::test]
    async fn test_send_receive() {
        let config = Arc::new(WebSocketConfig::default());
        let conn = Arc::new(WebSocketConnection::new(config));
        conn.set_state(ConnectionState::Open).await;
        
        // Send message
        let msg = Message::text("Hello");
        conn.send(msg.clone()).await.unwrap();
        
        // Receive message
        let received = conn.receive().await.unwrap().unwrap();
        match received {
            Message::Text(text) => assert_eq!(text, "Hello"),
            _ => panic!("Wrong message type"),
        }
        
        // Check metrics
        let metrics = conn.metrics().await;
        assert_eq!(metrics.messages_sent, 1);
        assert_eq!(metrics.messages_received, 1);
    }
    
    #[tokio::test]
    async fn test_connection_metadata() {
        let config = Arc::new(WebSocketConfig::default());
        let conn = WebSocketConnection::new(config);
        
        // Set different types of metadata
        conn.set_metadata("user_id_123").await;
        conn.set_metadata(42i32).await;
        conn.set_metadata(vec![1, 2, 3]).await;
        
        // Retrieve metadata
        assert_eq!(conn.get_metadata::<String>().await, Some("user_id_123".to_string()));
        assert_eq!(conn.get_metadata::<i32>().await, Some(42));
        assert_eq!(conn.get_metadata::<Vec<i32>>().await, Some(vec![1, 2, 3]));
        assert_eq!(conn.get_metadata::<f64>().await, None);
    }
    
    #[tokio::test]
    async fn test_connection_close() {
        let config = Arc::new(WebSocketConfig::default());
        let conn = WebSocketConnection::new(config);
        conn.set_state(ConnectionState::Open).await;
        
        // Close connection
        conn.close(CloseCode::Normal, "Test close").await.unwrap();
        
        assert_eq!(conn.state().await, ConnectionState::Closed);
        
        // Sending should fail
        let result = conn.send(Message::text("Should fail")).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_connection_pool() {
        let pool = ConnectionPool::new(10);
        let config = Arc::new(WebSocketConfig::default());
        
        // Add connections
        for _ in 0..5 {
            let conn = Arc::new(WebSocketConnection::new(config.clone()));
            pool.add(conn).await.unwrap();
        }
        
        assert_eq!(pool.count().await, 5);
        
        // Broadcast message
        let all_conns = pool.all().await;
        for conn in &all_conns {
            conn.set_state(ConnectionState::Open).await;
        }
        
        let sent = pool.broadcast(Message::text("Broadcast")).await.unwrap();
        assert_eq!(sent, 5);
        
        // Remove a connection
        let conn_id = all_conns[0].id();
        pool.remove(conn_id).await;
        assert_eq!(pool.count().await, 4);
    }
    
    #[tokio::test]
    async fn test_connection_pool_max_limit() {
        let pool = ConnectionPool::new(2);
        let config = Arc::new(WebSocketConfig::default());
        
        // Add up to limit
        for _ in 0..2 {
            let conn = Arc::new(WebSocketConnection::new(config.clone()));
            pool.add(conn).await.unwrap();
        }
        
        // Try to exceed limit
        let conn = Arc::new(WebSocketConnection::new(config));
        let result = pool.add(conn).await;
        assert!(result.is_err());
    }
}