"""Internal wrapper around the compiled Rust extension."""

from __future__ import annotations

import logging

logger = logging.getLogger(__name__)

try:
    # `_forzium` is produced by the PyO3 crate in ``module/pyo3-forzium``.
    from . import _forzium as bindings
except ImportError as exc:  # pragma: no cover - executed only when extension missing
    msg = (
        "Rust extension '_forzium' is not built. "
        "Run `cd module/pyo3-forzium && maturin develop` first."
    )
    logger.error(msg)
    raise ImportError(msg) from exc

validate_buffer_size = bindings.validate_buffer_size
validate_utf8_string = bindings.validate_utf8_string
validate_u8_range = bindings.validate_u8_range
PyRouteMatcher = bindings.PyRouteMatcher
request = bindings.request
response = bindings.response
validation = bindings.validation
dependencies = bindings.dependencies

__all__ = [
    # Re-exported from Rust
    "validate_buffer_size",
    "validate_utf8_string",
    "validate_u8_range",
    "PyRouteMatcher",
    "request",
    "response",
    "validation",
    "dependencies",
    
]