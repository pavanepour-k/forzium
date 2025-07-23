"""
Internal module for Rust FFI bindings.
DO NOT import directly - use forzium.__init__ instead.
"""
from ._forzium import *

__all__ = [
    # Re-exported from Rust
    "validate_buffer_size",
    "validate_utf8_string", 
    "validate_u8_range",
    "PyRouteMatcher",
]

logger.error(
        "Please ensure the Rust library is built: cd module/pyo3-forzium && maturin develop"
)