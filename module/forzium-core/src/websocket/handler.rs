//! # WEBSOCKET HANDLER SYSTEM
//!
//! **EVENT-DRIVEN HANDLER WITH ASYNC TRAIT SUPPORT**
//!
//! This module provides the handler trait and implementations for WebSocket events,
//! enabling flexible and performant event handling.

use super::{WebSocketConnection, Message, CloseCode, WebSocketError};
use std::sync::Arc;

/// **HANDLER RESULT TYPE**
pub type HandlerResult = Result<(), WebSocketError>;

/// **WEBSOCKET HANDLER TRAIT**
///
/// **PURPOSE**: Define the interface for handling WebSocket events.
/// **GUARANTEE**: All methods are async and thread-safe.
#[async_trait::async_trait]
pub trait WebSocketHandler: Send + Sync {
    /// **ON CONNECT**
    ///
    /// Called when a new WebSocket connection is established.
    ///
    /// **PARAMETERS**:
    /// - `connection: Arc<WebSocketConnection>` - The new connection
    ///
    /// **RETURNS**:
    /// - `Ok(())` - Accept the connection
    /// - `Err(WebSocketError)` - Reject the connection
    async fn on_connect(&self, connection: Arc<WebSocketConnection>) -> HandlerResult {
        log::debug!("WebSocket connected: {}", connection.id());
        Ok(())
    }
    
    /// **ON MESSAGE**
    ///
    /// Called when a message is received from the client.
    ///
    /// **PARAMETERS**:
    /// - `connection: Arc<WebSocketConnection>` - The connection
    /// - `message: Message` - The received message
    ///
    /// **RETURNS**:
    /// - `Ok(())` - Message processed successfully
    /// - `Err(WebSocketError)` - Error processing message
    async fn on_message(&self, connection: Arc<WebSocketConnection>, message: Message) -> HandlerResult;
    
    /// **ON DISCONNECT**
    ///
    /// Called when a WebSocket connection is closed.
    ///
    /// **PARAMETERS**:
    /// - `connection: Arc<WebSocketConnection>` - The connection
    /// - `code: CloseCode` - The close code
    async fn on_disconnect(&self, connection: Arc<WebSocketConnection>, code: CloseCode) -> HandlerResult {
        log::debug!("WebSocket disconnected: {} ({:?})", connection.id(), code);
        Ok(())
    }
    
    /// **ON ERROR**
    ///
    /// Called when an error occurs on the connection.
    ///
    /// **PARAMETERS**:
    /// - `connection: Arc<WebSocketConnection>` - The connection
    /// - `error: WebSocketError` - The error
    async fn on_error(&self, connection: Arc<WebSocketConnection>, error: WebSocketError) -> HandlerResult {
        log::error!("WebSocket error on {}: {}", connection.id(), error);
        Ok(())
    }
    
    /// **ON PING**
    ///
    /// Called when a ping is received. Default implementation sends pong.
    async fn on_ping(&self, connection: Arc<WebSocketConnection>, data: bytes::Bytes) -> HandlerResult {
        connection.send(Message::pong(data)).await?;
        Ok(())
    }
    
    /// **ON PONG**
    ///
    /// Called when a pong is received. Default implementation updates connection state.
    async fn on_pong(&self, connection: Arc<WebSocketConnection>, data: bytes::Bytes) -> HandlerResult {
        connection.handle_pong(data).await;
        Ok(())
    }
}

/// **ECHO HANDLER**
///
/// Simple handler that echoes all messages back to the sender.
pub struct EchoHandler;

#[async_trait::async_trait]
impl WebSocketHandler for EchoHandler {
    async fn on_message(&self, connection: Arc<WebSocketConnection>, message: Message) -> HandlerResult {
        // Echo back all data messages
        match &message {
            Message::Text(_) | Message::Binary(_) => {
                connection.send(message).await?;
            }
            Message::Ping(data) => {
                self.on_ping(connection, data.clone()).await?;
            }
            Message::Pong(data) => {
                self.on_pong(connection, data.clone()).await?;
            }
            Message::Close(_) => {
                // Connection will be closed by the manager
            }
        }
        Ok(())
    }
}

/// **BROADCAST HANDLER**
///
/// Handler that broadcasts all messages to all connected clients.
pub struct BroadcastHandler {
    manager: Arc<crate::websocket::WebSocketManager>,
}

impl BroadcastHandler {
    pub fn new(manager: Arc<crate::websocket::WebSocketManager>) -> Self {
        Self { manager }
    }
}

#[async_trait::async_trait]
impl WebSocketHandler for BroadcastHandler {
    async fn on_message(&self, _connection: Arc<WebSocketConnection>, message: Message) -> HandlerResult {
        // Broadcast data messages to all connections
        match message {
            Message::Text(_) | Message::Binary(_) => {
                self.manager.broadcast(message).await?;
            }
            _ => {} // Ignore control messages
        }
        Ok(())
    }
}

/// **ROUTED HANDLER**
///
/// Handler that routes messages based on content or type.
pub struct RoutedHandler {
    routes: std::collections::HashMap<String, Box<dyn WebSocketHandler>>,
    default_handler: Box<dyn WebSocketHandler>,
}

impl RoutedHandler {
    /// Create a new routed handler
    pub fn new(default_handler: Box<dyn WebSocketHandler>) -> Self {
        Self {
            routes: std::collections::HashMap::new(),
            default_handler,
        }
    }
    
    /// Add a route
    pub fn route(mut self, pattern: impl Into<String>, handler: Box<dyn WebSocketHandler>) -> Self {
        self.routes.insert(pattern.into(), handler);
        self
    }
    
    /// Extract route from message
    fn extract_route(&self, message: &Message) -> Option<String> {
        match message {
            Message::Text(text) => {
                // Simple JSON route extraction
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                    json.get("route")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[async_trait::async_trait]
impl WebSocketHandler for RoutedHandler {
    async fn on_connect(&self, connection: Arc<WebSocketConnection>) -> HandlerResult {
        self.default_handler.on_connect(connection).await
    }
    
    async fn on_message(&self, connection: Arc<WebSocketConnection>, message: Message) -> HandlerResult {
        // Extract route and delegate to appropriate handler
        if let Some(route) = self.extract_route(&message) {
            if let Some(handler) = self.routes.get(&route) {
                return handler.on_message(connection, message).await;
            }
        }
        
        // Fall back to default handler
        self.default_handler.on_message(connection, message).await
    }
    
    async fn on_disconnect(&self, connection: Arc<WebSocketConnection>, code: CloseCode) -> HandlerResult {
        self.default_handler.on_disconnect(connection, code).await
    }
}

/// **AUTHENTICATED HANDLER**
///
/// Handler wrapper that requires authentication before processing messages.
pub struct AuthenticatedHandler<H: WebSocketHandler> {
    inner: H,
    auth_timeout: std::time::Duration,
}

impl<H: WebSocketHandler> AuthenticatedHandler<H> {
    pub fn new(inner: H, auth_timeout: std::time::Duration) -> Self {
        Self { inner, auth_timeout }
    }
    
    async fn is_authenticated(&self, connection: &Arc<WebSocketConnection>) -> bool {
        connection.get_metadata::<bool>().await.unwrap_or(false)
    }
    
    async fn authenticate(&self, connection: &Arc<WebSocketConnection>, message: &Message) -> bool {
        // Simple token-based authentication
        if let Message::Text(text) = message {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                if let Some(token) = json.get("auth_token").and_then(|v| v.as_str()) {
                    // In production, verify token properly
                    if token == "valid_token" {
                        connection.set_metadata(true).await;
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[async_trait::async_trait]
impl<H: WebSocketHandler> WebSocketHandler for AuthenticatedHandler<H> {
    async fn on_connect(&self, connection: Arc<WebSocketConnection>) -> HandlerResult {
        // Set authentication deadline
        let deadline = std::time::Instant::now() + self.auth_timeout;
        connection.set_metadata(deadline).await;
        
        self.inner.on_connect(connection).await
    }
    
    async fn on_message(&self, connection: Arc<WebSocketConnection>, message: Message) -> HandlerResult {
        // Check if authenticated
        if !self.is_authenticated(&connection).await {
            // Try to authenticate with this message
            if self.authenticate(&connection, &message).await {
                // Send authentication success
                connection.send(Message::text(r#"{"status":"authenticated"}"#)).await?;
                return Ok(());
            }
            
            // Check deadline
            if let Some(deadline) = connection.get_metadata::<std::time::Instant>().await {
                if std::time::Instant::now() > deadline {
                    return Err(WebSocketError::HandlerError("Authentication timeout".to_string()));
                }
            }
            
            // Not authenticated yet
            connection.send(Message::text(r#"{"error":"not_authenticated"}"#)).await?;
            return Ok(());
        }
        
        // Authenticated, process normally
        self.inner.on_message(connection, message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::websocket::WebSocketConfig;
    
    #[tokio::test]
    async fn test_echo_handler() {
        let handler = EchoHandler;
        let config = Arc::new(WebSocketConfig::default());
        let conn = Arc::new(WebSocketConnection::new(config));
        conn.set_state(crate::websocket::ConnectionState::Open).await;
        
        // Test text message echo
        let text_msg = Message::text("Hello");
        handler.on_message(conn.clone(), text_msg.clone()).await.unwrap();
        
        // Should have echoed the message
        let received = conn.receive().await.unwrap().unwrap();
        match received {
            Message::Text(text) => assert_eq!(text, "Hello"),
            _ => panic!("Wrong message type"),
        }
    }
    
    #[tokio::test]
    async fn test_routed_handler() {
        let echo_handler = Box::new(EchoHandler);
        let default_handler = Box::new(EchoHandler);
        
        let routed = RoutedHandler::new(default_handler)
            .route("echo", echo_handler);
        
        let config = Arc::new(WebSocketConfig::default());
        let conn = Arc::new(WebSocketConnection::new(config));
        conn.set_state(crate::websocket::ConnectionState::Open).await;
        
        // Test routed message
        let routed_msg = Message::text(r#"{"route":"echo","data":"test"}"#);
        routed.on_message(conn.clone(), routed_msg).await.unwrap();
        
        // Test unrouted message
        let unrouted_msg = Message::text("plain text");
        routed.on_message(conn.clone(), unrouted_msg).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_authenticated_handler() {
        let inner = EchoHandler;
        let auth_handler = AuthenticatedHandler::new(
            inner,
            std::time::Duration::from_secs(5)
        );
        
        let config = Arc::new(WebSocketConfig::default());
        let conn = Arc::new(WebSocketConnection::new(config));
        conn.set_state(crate::websocket::ConnectionState::Open).await;
        
        // Connect
        auth_handler.on_connect(conn.clone()).await.unwrap();
        
        // Try to send message without auth
        let msg = Message::text("Hello");
        auth_handler.on_message(conn.clone(), msg).await.unwrap();
        
        // Should receive error
        let response = conn.receive().await.unwrap().unwrap();
        match response {
            Message::Text(text) => assert!(text.contains("not_authenticated")),
            _ => panic!("Wrong response type"),
        }
        
        // Authenticate
        let auth_msg = Message::text(r#"{"auth_token":"valid_token"}"#);
        auth_handler.on_message(conn.clone(), auth_msg).await.unwrap();
        
        // Should receive success
        let response = conn.receive().await.unwrap().unwrap();
        match response {
            Message::Text(text) => assert!(text.contains("authenticated")),
            _ => panic!("Wrong response type"),
        }
        
        // Now messages should work
        let msg = Message::text("Hello after auth");
        auth_handler.on_message(conn.clone(), msg).await.unwrap();
        
        let response = conn.receive().await.unwrap().unwrap();
        match response {
            Message::Text(text) => assert_eq!(text, "Hello after auth"),
            _ => panic!("Wrong response type"),
        }
    }
}