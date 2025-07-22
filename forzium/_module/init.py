"""
Internal module for Rust FFI bindings.
DO NOT import directly - use forzium.init instead.
"""
from ._forzium import *
all = [
# Re-exported from Rust
"validate_buffer_size",
"validate_utf8_string",
"validate_u8_range",
"PyRouteMatcher",
]
