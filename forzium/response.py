"""
Response Module - HTTP response handling with Rust acceleration

Provides FastAPI-compatible response objects with high-performance
serialization powered by Rust FFI functions.
"""

from typing import Dict, Any, Optional, Union, List
from dataclasses import dataclass, field
import json
import logging

from ._rust_lib import response as _rust_response
from .exceptions import ValidationError

logger = logging.getLogger(__name__)


@dataclass
class Response:
    """
    HTTP Response wrapper with Rust-powered serialization.
    
    This class provides a Pythonic interface while leveraging
    Rust for high-performance response building and serialization.
    
    Attributes:
        status_code: HTTP status code (100-599)
        headers: Response headers dictionary
        body: Response body (various types supported)
    """
    
    status_code: int = 200
    headers: Dict[str, str] = field(default_factory=dict)
    body: Optional[Union[str, bytes, Dict[str, Any], List[Any]]] = None
    
    def __post_init__(self):
        """Validate response parameters."""
        if not 100 <= self.status_code <= 599:
            raise ValidationError(
                message=f"Invalid status code: {self.status_code}",
                field="status_code",
                value=self.status_code
            )
        logger.debug(f"Created response with status {self.status_code}")
    
    @classmethod
    def json(
        cls,
        data: Union[Dict[str, Any], List[Any]],
        status: int = 200,
        headers: Optional[Dict[str, str]] = None
    ) -> 'Response':
        """
        Create a JSON response.
        
        Args:
            data: JSON-serializable data
            status: HTTP status code
            headers: Additional headers
            
        Returns:
            Response instance
        """
        response_headers = {"Content-Type": "application/json"}
        if headers:
            response_headers.update(headers)
            
        return cls(
            status_code=status,
            headers=response_headers,
            body=data
        )
    
    @classmethod
    def text(
        cls,
        text: str,
        status: int = 200,
        headers: Optional[Dict[str, str]] = None
    ) -> 'Response':
        """
        Create a plain text response.
        
        Args:
            text: Text content
            status: HTTP status code
            headers: Additional headers
            
        Returns:
            Response instance
        """
        response_headers = {"Content-Type": "text/plain; charset=utf-8"}
        if headers:
            response_headers.update(headers)
            
        return cls(
            status_code=status,
            headers=response_headers,
            body=text
        )
    
    @classmethod
    def binary(
        cls,
        data: bytes,
        status: int = 200,
        content_type: str = "application/octet-stream",
        headers: Optional[Dict[str, str]] = None
    ) -> 'Response':
        """
        Create a binary response.
        
        Args:
            data: Binary data
            status: HTTP status code
            content_type: MIME type for the data
            headers: Additional headers
            
        Returns:
            Response instance
        """
        response_headers = {"Content-Type": content_type}
        if headers:
            response_headers.update(headers)
            
        return cls(
            status_code=status,
            headers=response_headers,
            body=data
        )
    
    @classmethod
    def error(
        cls,
        message: str,
        status: int = 400,
        details: Optional[Dict[str, Any]] = None,
        headers: Optional[Dict[str, str]] = None
    ) -> 'Response':
        """
        Create an error response.
        
        Args:
            message: Error message
            status: HTTP error status code
            details: Additional error details
            headers: Additional headers
            
        Returns:
            Response instance
        """
        error_body = {
            "error": message,
            "status": status
        }
        if details:
            error_body["details"] = details
            
        return cls.json(error_body, status=status, headers=headers)
    
    def to_rust(self) -> '_rust_response.PyHttpResponse':
        """
        Convert to Rust response object for optimized serialization.
        
        Returns:
            PyHttpResponse instance
            
        Raises:
            ValidationError: If response cannot be serialized
        """
        builder = _rust_response.PyResponseBuilder()
        
        # Set status
        try:
            builder.status(self.status_code)
        except ValueError as e:
            raise ValidationError(
                message=f"Invalid status code: {e}",
                field="status_code", 
                value=self.status_code
            )
        
        # Set headers
        for key, value in self.headers.items():
            try:
                builder.header(key, value)
            except ValueError as e:
                logger.warning(f"Invalid header {key}: {e}")
        
        # Set body based on type
        if self.body is None:
            pass  # Empty body
        elif isinstance(self.body, bytes):
            builder.binary_body(self.body)
        elif isinstance(self.body, str):
            builder.text_body(self.body)
        elif isinstance(self.body, (dict, list)):
            # Convert to JSON using Rust
            builder.json_body(self.body)
        else:
            # Try to JSON serialize other types
            try:
                json_data = json.dumps(self.body)
                builder.text_body(json_data)
                if "Content-Type" not in self.headers:
                    builder.header("Content-Type", "application/json")
            except (TypeError, ValueError) as e:
                raise ValidationError(
                    message=f"Cannot serialize body: {e}",
                    field="body"
                )
        
        return builder.build()
    
    def is_json(self) -> bool:
        """Check if response has JSON content type."""
        content_type = self.headers.get("Content-Type", "")
        return "application/json" in content_type.lower()
    
    def is_text(self) -> bool:
        """Check if response has text content type."""
        content_type = self.headers.get("Content-Type", "")
        return "text/" in content_type.lower()
    
    def is_binary(self) -> bool:
        """Check if response has binary content."""
        return isinstance(self.body, bytes)
    
    def body_string(self) -> str:
        """Get body as string."""
        if isinstance(self.body, bytes):
            return self.body.decode('utf-8')
        elif isinstance(self.body, str):
            return self.body
        elif self.body is None:
            return ""
        else:
            return json.dumps(self.body)
    
    def __repr__(self) -> str:
        body_type = type(self.body).__name__ if self.body else "None"
        return f"Response(status={self.status_code}, type={body_type})"


# Response utilities using Rust functions directly
def json_response(data: Dict[str, Any], status: int = 200) -> Any:
    """
    Create a JSON response using Rust serialization.
    
    This bypasses the Python Response class for maximum performance.
    """
    try:
        return _rust_response.json_response(status, data)
    except ValueError as e:
        raise ValidationError(f"Failed to create JSON response: {e}")


def text_response(text: str, status: int = 200) -> Any:
    """
    Create a text response using Rust.
    
    This bypasses the Python Response class for maximum performance.
    """
    try:
        return _rust_response.text_response(status, text)
    except ValueError as e:
        raise ValidationError(f"Failed to create text response: {e}")


def binary_response(data: bytes, status: int = 200) -> Any:
    """
    Create a binary response using Rust.
    
    This bypasses the Python Response class for maximum performance.
    """
    try:
        return _rust_response.binary_response(status, data)
    except ValueError as e:
        raise ValidationError(f"Failed to create binary response: {e}")


# Common response shortcuts
def ok(data: Any = None, headers: Optional[Dict[str, str]] = None) -> Response:
    """Create a 200 OK response."""
    if data is None:
        return Response(status_code=200, headers=headers or {})
    elif isinstance(data, str):
        return Response.text(data, headers=headers)
    else:
        return Response.json(data, headers=headers)


def created(data: Any = None, location: Optional[str] = None) -> Response:
    """Create a 201 Created response."""
    headers = {}
    if location:
        headers["Location"] = location
    
    if data is None:
        return Response(status_code=201, headers=headers)
    else:
        return Response.json(data, status=201, headers=headers)


def no_content() -> Response:
    """Create a 204 No Content response."""
    return Response(status_code=204)


def bad_request(message: str, details: Optional[Dict[str, Any]] = None) -> Response:
    """Create a 400 Bad Request response."""
    return Response.error(message, status=400, details=details)


def unauthorized(message: str = "Unauthorized") -> Response:
    """Create a 401 Unauthorized response."""
    return Response.error(message, status=401)


def forbidden(message: str = "Forbidden") -> Response:
    """Create a 403 Forbidden response."""
    return Response.error(message, status=403)


def not_found(message: str = "Not Found") -> Response:
    """Create a 404 Not Found response."""
    return Response.error(message, status=404)


def internal_server_error(message: str = "Internal Server Error") -> Response:
    """Create a 500 Internal Server Error response."""
    return Response.error(message, status=500)


__all__ = [
    # Main class
    "Response",
    # Direct Rust functions
    "json_response",
    "text_response", 
    "binary_response",
    # Convenience functions
    "ok",
    "created",
    "no_content",
    "bad_request",
    "unauthorized",
    "forbidden",
    "not_found",
    "internal_server_error",
]