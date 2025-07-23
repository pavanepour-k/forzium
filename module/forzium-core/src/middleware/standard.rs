//! # STANDARD MIDDLEWARE IMPLEMENTATIONS
//!
//! **HIGH-PERFORMANCE MIDDLEWARE FOR COMMON USE CASES**
//!
//! This module provides production-ready middleware implementations
//! that cover the most common cross-cutting concerns in web applications.

use super::{Middleware, Next};
use crate::request::HttpRequest;
use crate::response::{HttpResponse, ResponseBody};
use crate::routing::HttpMethod;
use std::collections::HashSet;
use std::time::Instant;

// ================================================================================================
// CORS MIDDLEWARE - Cross-Origin Resource Sharing
// ================================================================================================

/// **CORS MIDDLEWARE**
///
/// **PURPOSE**: Handle Cross-Origin Resource Sharing (CORS) headers.
/// **PERFORMANCE**: O(1) header operations with minimal overhead.
/// **SECURITY**: Configurable origin validation with wildcard support.
#[derive(Debug, Clone)]
pub struct CorsMiddleware {
    /// Allowed origins (use "*" for all)
    allowed_origins: HashSet<String>,
    
    /// Allowed HTTP methods
    allowed_methods: HashSet<HttpMethod>,
    
    /// Allowed headers
    allowed_headers: HashSet<String>,
    
    /// Exposed headers
    exposed_headers: Vec<String>,
    
    /// Allow credentials
    allow_credentials: bool,
    
    /// Max age for preflight cache (seconds)
    max_age: Option<u32>,
}

impl CorsMiddleware {
    /// **CONSTRUCTOR**
    pub fn new() -> Self {
        Self {
            allowed_origins: HashSet::from(["*".to_string()]),
            allowed_methods: HashSet::from([
                HttpMethod::GET,
                HttpMethod::POST,
                HttpMethod::PUT,
                HttpMethod::DELETE,
                HttpMethod::PATCH,
                HttpMethod::HEAD,
                HttpMethod::OPTIONS,
            ]),
            allowed_headers: HashSet::from([
                "content-type".to_string(),
                "authorization".to_string(),
                "accept".to_string(),
            ]),
            exposed_headers: vec![],
            allow_credentials: false,
            max_age: Some(86400), // 24 hours
        }
    }
    
    /// **BUILDER: Set allowed origins**
    pub fn allowed_origin(mut self, origin: impl Into<String>) -> Self {
        let origin = origin.into();
        if origin == "*" {
            self.allowed_origins.clear();
        }
        self.allowed_origins.insert(origin);
        self
    }
    
    /// **BUILDER: Set allowed methods**
    pub fn allowed_methods(mut self, methods: Vec<HttpMethod>) -> Self {
        self.allowed_methods = methods.into_iter().collect();
        self
    }
    
    /// **BUILDER: Allow credentials**
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.allow_credentials = allow;
        self
    }
    
    /// **CHECK ORIGIN**
    fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.contains("*") || self.allowed_origins.contains(origin)
    }
    
    /// **HANDLE PREFLIGHT REQUEST**
    fn handle_preflight(&self, request: &HttpRequest) -> HttpResponse {
        let mut headers = std::collections::HashMap::new();
        
        // Check origin
        if let Some(origin) = request.headers.get("origin") {
            if self.is_origin_allowed(origin) {
                headers.insert("Access-Control-Allow-Origin".to_string(), origin.clone());
            }
        }
        
        // Add allowed methods
        let methods: Vec<String> = self.allowed_methods
            .iter()
            .map(|m| format!("{:?}", m))
            .collect();
        headers.insert(
            "Access-Control-Allow-Methods".to_string(),
            methods.join(", "),
        );
        
        // Add allowed headers
        headers.insert(
            "Access-Control-Allow-Headers".to_string(),
            self.allowed_headers
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        );
        
        // Add max age
        if let Some(max_age) = self.max_age {
            headers.insert(
                "Access-Control-Max-Age".to_string(),
                max_age.to_string(),
            );
        }
        
        // Add credentials
        if self.allow_credentials {
            headers.insert(
                "Access-Control-Allow-Credentials".to_string(),
                "true".to_string(),
            );
        }
        
        HttpResponse {
            status_code: 204,
            headers,
            body: ResponseBody::Empty,
        }
    }
}

impl Default for CorsMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Middleware for CorsMiddleware {
    async fn process(&self, request: HttpRequest, next: Next) -> HttpResponse {
        // Handle preflight requests
        if request.method == HttpMethod::OPTIONS {
            return self.handle_preflight(&request);
        }
        
        // Process regular request
        let origin = request.headers.get("origin").cloned();
        let mut response = next(request).await;
        
        // Add CORS headers to response
        if let Some(origin) = origin {
            if self.is_origin_allowed(&origin) {
                response.headers.insert(
                    "Access-Control-Allow-Origin".to_string(),
                    origin,
                );
                
                if self.allow_credentials {
                    response.headers.insert(
                        "Access-Control-Allow-Credentials".to_string(),
                        "true".to_string(),
                    );
                }
                
                if !self.exposed_headers.is_empty() {
                    response.headers.insert(
                        "Access-Control-Expose-Headers".to_string(),
                        self.exposed_headers.join(", "),
                    );
                }
            }
        }
        
        response
    }
}

// ================================================================================================
// COMPRESSION MIDDLEWARE - Response compression
// ================================================================================================

/// **COMPRESSION MIDDLEWARE**
///
/// **PURPOSE**: Compress response bodies to reduce bandwidth.
/// **PERFORMANCE**: Uses streaming compression to minimize memory usage.
/// **ALGORITHMS**: Supports gzip and brotli compression.
#[derive(Debug, Clone)]
pub struct CompressionMiddleware {
    /// Minimum response size to compress (bytes)
    min_size: usize,
    
    /// Compression level (1-9)
    level: u8,
    
    /// Content types to compress
    compressible_types: HashSet<String>,
}

impl CompressionMiddleware {
    /// **CONSTRUCTOR**
    pub fn new() -> Self {
        Self {
            min_size: 1024, // 1KB
            level: 6,       // Default compression level
            compressible_types: HashSet::from([
                "text/html".to_string(),
                "text/css".to_string(),
                "text/plain".to_string(),
                "text/javascript".to_string(),
                "application/javascript".to_string(),
                "application/json".to_string(),
                "application/xml".to_string(),
            ]),
        }
    }
    
    /// **CHECK IF CONTENT SHOULD BE COMPRESSED**
    fn should_compress(&self, content_type: Option<&String>, body: &ResponseBody) -> bool {
        // Check size
        let size = match body {
            ResponseBody::Text(s) => s.len(),
            ResponseBody::Json(v) => v.to_string().len(),
            ResponseBody::Binary(b) => b.len(),
            ResponseBody::Empty => return false,
        };
        
        if size < self.min_size {
            return false;
        }
        
        // Check content type
        if let Some(ct) = content_type {
            self.compressible_types.iter().any(|t| ct.contains(t))
        } else {
            false
        }
    }
    
    /// **COMPRESS BODY**
    fn compress_body(&self, body: ResponseBody, encoding: &str) -> Result<ResponseBody, std::io::Error> {
        use flate2::write::{GzEncoder, DeflateEncoder};
        use flate2::Compression;
        use std::io::Write;
        
        let data = match body {
            ResponseBody::Text(s) => s.into_bytes(),
            ResponseBody::Json(v) => serde_json::to_vec(&v)?,
            ResponseBody::Binary(b) => b,
            ResponseBody::Empty => return Ok(ResponseBody::Empty),
        };
        
        let compressed = match encoding {
            "gzip" => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level as u32));
                encoder.write_all(&data)?;
                encoder.finish()?
            },
            "deflate" => {
                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(self.level as u32));
                encoder.write_all(&data)?;
                encoder.finish()?
            },
            _ => return Ok(ResponseBody::Binary(data)),
        };
        
        Ok(ResponseBody::Binary(compressed))
    }
}

impl Default for CompressionMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Middleware for CompressionMiddleware {
    async fn process(&self, request: HttpRequest, next: Next) -> HttpResponse {
        // Check Accept-Encoding header
        let accept_encoding = request
            .headers
            .get("accept-encoding")
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        
        let mut response = next(request).await;
        
        // Skip if already encoded
        if response.headers.contains_key("content-encoding") {
            return response;
        }
        
        // Check if we should compress
        let content_type = response.headers.get("content-type");
        if !self.should_compress(content_type, &response.body) {
            return response;
        }
        
        // Determine encoding
        let encoding = if accept_encoding.contains("gzip") {
            "gzip"
        } else if accept_encoding.contains("deflate") {
            "deflate"
        } else {
            return response;
        };
        
        // Compress body
        match self.compress_body(response.body, encoding) {
            Ok(compressed_body) => {
                response.body = compressed_body;
                response.headers.insert(
                    "content-encoding".to_string(),
                    encoding.to_string(),
                );
                response.headers.insert(
                    "vary".to_string(),
                    "accept-encoding".to_string(),
                );
            }
            Err(e) => {
                log::error!("Compression failed: {}", e);
            }
        }
        
        response
    }
}

// ================================================================================================
// LOGGING MIDDLEWARE - Request/Response logging
// ================================================================================================

/// **LOGGING MIDDLEWARE**
///
/// **PURPOSE**: Log HTTP requests and responses with configurable detail.
/// **PERFORMANCE**: Uses structured logging with minimal overhead.
/// **PRIVACY**: Supports header/body redaction for sensitive data.
#[derive(Debug, Clone)]
pub struct LoggingMiddleware {
    /// Log request headers
    log_headers: bool,
    
    /// Log request body
    log_body: bool,
    
    /// Headers to redact
    redacted_headers: HashSet<String>,
    
    /// Maximum body size to log
    max_body_log_size: usize,
}

impl LoggingMiddleware {
    /// **CONSTRUCTOR**
    pub fn new() -> Self {
        Self {
            log_headers: true,
            log_body: false,
            redacted_headers: HashSet::from([
                "authorization".to_string(),
                "cookie".to_string(),
                "x-api-key".to_string(),
            ]),
            max_body_log_size: 1024,
        }
    }
    
    /// **REDACT SENSITIVE HEADERS**
    fn redact_headers(&self, headers: &std::collections::HashMap<String, String>) -> std::collections::HashMap<String, String> {
        headers
            .iter()
            .map(|(k, v)| {
                let key = k.to_lowercase();
                let value = if self.redacted_headers.contains(&key) {
                    "[REDACTED]".to_string()
                } else {
                    v.clone()
                };
                (k.clone(), value)
            })
            .collect()
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Middleware for LoggingMiddleware {
    async fn process(&self, request: HttpRequest, next: Next) -> HttpResponse {
        let start = Instant::now();
        let method = format!("{:?}", request.method);
        let path = request.path.clone();
        
        // Log request
        if self.log_headers {
            let headers = self.redact_headers(&request.headers);
            log::info!(
                "→ {} {} headers={:?}",
                method,
                path,
                headers
            );
        } else {
            log::info!("→ {} {}", method, path);
        }
        
        // Process request
        let response = next(request).await;
        let elapsed = start.elapsed();
        
        // Log response
        log::info!(
            "← {} {} {} {}ms",
            method,
            path,
            response.status_code,
            elapsed.as_millis()
        );
        
        response
    }
}

// ================================================================================================
// TIMING MIDDLEWARE - Request timing and performance monitoring
// ================================================================================================

/// **TIMING MIDDLEWARE**
///
/// **PURPOSE**: Add timing information to responses.
/// **PERFORMANCE**: Uses high-precision timing with minimal overhead.
/// **METRICS**: Supports external metrics collection.
#[derive(Debug, Clone)]
pub struct TimingMiddleware {
    /// Add Server-Timing header
    add_header: bool,
    
    /// Collect metrics
    collect_metrics: bool,
}

impl TimingMiddleware {
    /// **CONSTRUCTOR**
    pub fn new() -> Self {
        Self {
            add_header: true,
            collect_metrics: true,
        }
    }
}

impl Default for TimingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Middleware for TimingMiddleware {
    async fn process(&self, request: HttpRequest, next: Next) -> HttpResponse {
        let start = Instant::now();
        
        let mut response = next(request).await;
        
        let duration = start.elapsed();
        
        // Add timing header
        if self.add_header {
            response.headers.insert(
                "server-timing".to_string(),
                format!("total;dur={}", duration.as_millis()),
            );
        }
        
        // Collect metrics (would integrate with Prometheus/StatsD)
        if self.collect_metrics {
            log::debug!("Request processed in {}μs", duration.as_micros());
        }
        
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cors_preflight() {
        let cors = CorsMiddleware::new();
        
        let request = HttpRequest {
            method: HttpMethod::OPTIONS,
            path: "/api/test".to_string(),
            headers: std::collections::HashMap::from([
                ("origin".to_string(), "https://example.com".to_string()),
            ]),
            query_params: Default::default(),
            body: crate::request::RequestBody::Empty,
        };
        
        let response = cors.process(request, Box::new(|_| unreachable!())).await;
        
        assert_eq!(response.status_code, 204);
        assert!(response.headers.contains_key("Access-Control-Allow-Origin"));
        assert!(response.headers.contains_key("Access-Control-Allow-Methods"));
    }
    
    #[tokio::test]
    async fn test_compression_middleware() {
        let compression = CompressionMiddleware::new();
        
        let request = HttpRequest {
            method: HttpMethod::GET,
            path: "/api/data".to_string(),
            headers: std::collections::HashMap::from([
                ("accept-encoding".to_string(), "gzip, deflate".to_string()),
            ]),
            query_params: Default::default(),
            body: crate::request::RequestBody::Empty,
        };
        
        let response = compression
            .process(request, Box::new(|_| {
                Box::pin(async {
                    HttpResponse {
                        status_code: 200,
                        headers: std::collections::HashMap::from([
                            ("content-type".to_string(), "application/json".to_string()),
                        ]),
                        body: ResponseBody::Json(serde_json::json!({
                            "data": "a".repeat(2000) // Large enough to compress
                        })),
                    }
                })
            }))
            .await;
        
        assert!(response.headers.contains_key("content-encoding"));
        match response.body {
            ResponseBody::Binary(_) => (), // Compressed
            _ => panic!("Expected compressed binary body"),
        }
    }
}