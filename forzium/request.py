"""
Request Module - HTTP request handling with Rust acceleration

Provides FastAPI-compatible request objects with high-performance
parsing powered by Rust FFI functions.
"""

from typing import Dict, Any, Optional, Union, List
from dataclasses import dataclass, field
import json
import logging

from ._rust_lib import request as _rust_request
from .exceptions import ValidationError

logger = logging.getLogger(__name__)


@dataclass
class Request:
    """
    HTTP Request representation with Rust-powered parsing.
    
    Attributes:
        method: HTTP method (GET, POST, etc.)
        path: Request path/URL
        headers: HTTP headers dictionary
        query_params: Parsed query parameters
        body: Request body (raw bytes)
        path_params: Path parameters from routing
    """
    method: str
    path: str
    headers: Dict[str, str] = field(default_factory=dict)
    query_params: Dict[str, str] = field(default_factory=dict)
    body: Optional[bytes] = None
    path_params: Dict[str, str] = field(default_factory=dict)
    
    def __post_init__(self):
        """Parse query string if present in path."""
        if '?' in self.path:
            path_part, query_part = self.path.split('?', 1)
            self.path = path_part
            if query_part and not self.query_params:
                try:
                    self.query_params = _rust_request.parse_query_params(query_part)
                    logger.debug(f"Parsed {len(self.query_params)} query parameters")
                except Exception as e:
                    logger.warning(f"Failed to parse query string: {e}")
    
    @property
    def content_type(self) -> Optional[str]:
        """Get Content-Type header value."""
        return self.headers.get("content-type", self.headers.get("Content-Type"))
    
    def json(self) -> Any:
        """
        Parse request body as JSON using Rust parser.
        
        Returns:
            Parsed JSON data
            
        Raises:
            ValidationError: If body is not valid JSON
        """
        if not self.body:
            return None
            
        try:
            result = _rust_request.parse_json(self.body)
            logger.debug("Successfully parsed JSON body")
            return result
        except ValueError as e:
            logger.warning(f"JSON parsing failed: {e}")
            raise ValidationError(
                message=f"Invalid JSON body: {e}",
                field="body"
            )
    
    def form(self) -> Dict[str, str]:
        """
        Parse request body as form data using Rust parser.
        
        Returns:
            Dictionary of form fields
            
        Raises:
            ValidationError: If body is not valid form data
        """
        if not self.body:
            return {}
            
        try:
            result = _rust_request.parse_form(self.body)
            logger.debug(f"Successfully parsed form with {len(result)} fields")
            return result
        except ValueError as e:
            logger.warning(f"Form parsing failed: {e}")
            raise ValidationError(
                message=f"Invalid form data: {e}",
                field="body"
            )
    
    def get_header(self, name: str, default: Optional[str] = None) -> Optional[str]:
        """Get header value case-insensitively."""
        # Try exact match first
        if name in self.headers:
            return self.headers[name]
        
        # Try case-insensitive match
        lower_name = name.lower()
        for key, value in self.headers.items():
            if key.lower() == lower_name:
                return value
                
        return default
    
    def is_json(self) -> bool:
        """Check if request has JSON content type."""
        content_type = self.content_type
        return content_type is not None and "application/json" in content_type.lower()
    
    def is_form(self) -> bool:
        """Check if request has form content type."""
        content_type = self.content_type
        return content_type is not None and (
            "application/x-www-form-urlencoded" in content_type.lower() or
            "multipart/form-data" in content_type.lower()
        )
    
    def __repr__(self) -> str:
        body_size = len(self.body) if self.body else 0
        return (
            f"Request(method='{self.method}', path='{self.path}', "
            f"headers={len(self.headers)}, body_size={body_size})"
        )


class RequestHandler:
    """
    FastAPI-compatible request handler with dependency injection.
    
    This class provides routing and request handling functionality
    similar to FastAPI but with Rust-powered performance.
    """
    
    def __init__(self):
        from .routing import Router
        from .dependencies import DependencyInjector
        
        self.router = Router()
        self.injector = DependencyInjector()
        self.middleware: List[Any] = []
        logger.info("RequestHandler initialized")
    
    def route(self, path: str, methods: Optional[List[str]] = None):
        """
        Decorator for route registration.
        
        Args:
            path: URL path pattern (e.g., "/users/{id}")
            methods: List of HTTP methods (default: ["GET"])
            
        Returns:
            Decorator function
        """
        if methods is None:
            methods = ["GET"]
        
        def decorator(func):
            import asyncio
            from functools import wraps
            
            @wraps(func)
            async def wrapper(request: Request, **kwargs):
                # Inject dependencies
                deps = await self.injector.resolve_all(func)
                
                # Apply middleware
                for mw in self.middleware:
                    if asyncio.iscoroutinefunction(mw):
                        request = await mw(request)
                    else:
                        request = mw(request)
                
                # Merge path params into kwargs
                kwargs.update(request.path_params)
                
                # Call handler
                if asyncio.iscoroutinefunction(func):
                    return await func(request, **deps, **kwargs)
                else:
                    return func(request, **deps, **kwargs)
            
            # Register route for each method
            for method in methods:
                self.router.add_route(path, method, wrapper)
                logger.debug(f"Registered route: {method} {path}")
            
            return wrapper
        return decorator
    
    def add_middleware(self, middleware):
        """
        Add middleware to the request processing pipeline.
        
        Args:
            middleware: Callable that processes requests
        """
        self.middleware.append(middleware)
        logger.debug(f"Added middleware: {middleware.__name__}")
    
    async def handle_request(self, request: Request) -> Any:
        """
        Handle incoming HTTP request.
        
        Args:
            request: Request object to process
            
        Returns:
            Handler response
            
        Raises:
            ValueError: If no matching route found
        """
        try:
            handler, params = self.router.match(request.path, request.method)
            request.path_params = params
            logger.debug(f"Matched route: {request.method} {request.path}")
            return await handler(request)
        except ValueError as e:
            logger.warning(f"No route found: {request.method} {request.path}")
            raise
    
    def include_router(self, router, prefix: str = ""):
        """
        Include routes from another router.
        
        Args:
            router: Router instance to include
            prefix: URL prefix for included routes
        """
        self.router.include_router(router, prefix)
        logger.info(f"Included router with prefix: {prefix}")


# Middleware utilities
async def log_requests(request: Request) -> Request:
    """Example logging middleware."""
    logger.info(f"Request: {request.method} {request.path}")
    return request


async def parse_json_body(request: Request) -> Request:
    """Middleware to automatically parse JSON bodies."""
    if request.is_json() and request.body:
        try:
            # Pre-parse JSON for convenience
            request._parsed_json = request.json()
        except ValidationError:
            pass  # Let handler deal with invalid JSON
    return request


__all__ = [
    "Request",
    "RequestHandler",
    "log_requests",
    "parse_json_body",
]