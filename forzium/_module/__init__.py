"""
Forzium - High-Performance FastAPI Core Replacement

A Rust-powered Python web framework providing FastAPI-compatible APIs
with significant performance improvements for CPU-intensive operations.
"""

import logging
from typing import TYPE_CHECKING

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Version
__version__ = "0.1.0"
logger.info(f"Initializing {__name__} v{__version__}")

# Import Rust FFI functions with error handling
try:
    from ._module import (
        # Core validation functions
        validate_buffer_size,
        validate_utf8_string,
        validate_u8_range,
        # Routing components
        PyRouteMatcher,
        # Submodules
        request as _forzium_request,
        response as _forzium_response,
        validation as _forzium_validation,
        dependencies as _forzium_dependencies,
        __version__ as _forzium_version,
    )
    logger.debug(f"Rust library loaded successfully (version: {_forzium_version})")
except ImportError as e:
    logger.error(f"Failed to import Rust library: {e}")
    logger.error("Please ensure the Rust library is built: cd rust/bindings && maturin develop")
    raise

# Import Python modules
from .exceptions import (
    ProjectError,
    ValidationError,
    ProcessingError,
    TimeoutError,
    SystemError,
)
from .routing import Router
from .dependencies import DependencyInjector
from .request import Request, RequestHandler
from .response import Response

# Import validators wrapper
from .validators import (
    validate_buffer_size as py_validate_buffer_size,
    validate_utf8_string as py_validate_utf8_string,
    validate_u8_range as py_validate_u8_range,
)

# Public API
__all__ = [
    # Version
    "__version__",
    # Core Classes
    "Router",
    "DependencyInjector",
    "Request",
    "RequestHandler",
    "Response",
    # Exceptions
    "ProjectError",
    "ValidationError",
    "ProcessingError", 
    "TimeoutError",
    "SystemError",
    # Validation Functions (Python wrappers)
    "validate_buffer_size",
    "validate_utf8_string",
    "validate_u8_range",
    # Rust Components (for advanced usage)
    "PyRouteMatcher",
]

# Type checking support
if TYPE_CHECKING:
    from ._module import (
        PyBufferValidator,
        PyUtf8Validator,
        PyNumericValidator,
        PySchemaValidator,
        PyResponseBuilder,
        PyHttpResponse,
        PyDependencyResolver,
    )