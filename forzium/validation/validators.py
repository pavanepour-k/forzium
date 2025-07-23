"""
Validators Module - Python-friendly wrappers around Rust validators

This module provides high-level validation functions that wrap the
low-level Rust FFI functions with better error handling and type hints.
"""

from typing import Any, Optional, Dict
import logging

from ._module import (
    validate_buffer_size as _forzium_validate_buffer_size,
    validate_utf8_string as _forzium_validate_utf8_string,
    validate_u8_range as _forzium_validate_u8_range,
    validation as _forzium_validation,
)
from .exceptions import ValidationError

logger = logging.getLogger(__name__)

# Re-export from submodule
BufferValidator = _forzium_validation.PyBufferValidator
Utf8Validator = _forzium_validation.PyUtf8Validator
NumericValidator = _forzium_validation.PyNumericValidator
SchemaValidator = _forzium_validation.PySchemaValidator

# Constants
MAX_BUFFER_SIZE = 10_485_760  # 10MB
U8_MIN = 0
U8_MAX = 255


def validate_buffer_size(data: bytes) -> None:
    """
    Validate buffer size against maximum allowed limit.
    
    Args:
        data: Byte buffer to validate
        
    Raises:
        ValidationError: If buffer exceeds maximum size
        TypeError: If data is not bytes
    """
    if not isinstance(data, bytes):
        raise TypeError(f"Expected bytes, got {type(data).__name__}")
    
    try:
        _forzium_validate_buffer_size(data)
        logger.debug(f"Buffer validation successful: {len(data)} bytes")
    except ValueError as e:
        logger.warning(f"Buffer validation failed: {e}")
        raise ValidationError(
            message=str(e),
            field="buffer",
            value=f"{len(data)} bytes"
        )


def validate_utf8_string(data: bytes) -> str:
    """
    Validate byte sequence as UTF-8 and convert to string.
    
    Args:
        data: Byte sequence to validate
        
    Returns:
        Decoded UTF-8 string
        
    Raises:
        ValidationError: If not valid UTF-8
        TypeError: If data is not bytes
    """
    if not isinstance(data, bytes):
        raise TypeError(f"Expected bytes, got {type(data).__name__}")
    
    try:
        result = _forzium_validate_utf8_string(data)
        logger.debug(f"UTF-8 validation successful: {len(result)} chars")
        return result
    except ValueError as e:
        logger.warning(f"UTF-8 validation failed: {e}")
        raise ValidationError(
            message=str(e),
            field="utf8_string",
            value=f"{len(data)} bytes"
        )


def validate_u8_range(value: int) -> int:
    """
    Validate integer is in u8 range (0-255).
    
    Args:
        value: Integer to validate
        
    Returns:
        Validated integer
        
    Raises:
        ValidationError: If value outside u8 range
        TypeError: If value is not int
    """
    if not isinstance(value, int):
        raise TypeError(f"Expected int, got {type(value).__name__}")
    
    # Python-side pre-validation
    if value < U8_MIN or value > U8_MAX:
        raise ValidationError(
            message=f"Value {value} outside u8 range ({U8_MIN}-{U8_MAX})",
            field="u8_value",
            value=value
        )
    
    try:
        _forzium_validate_u8_range(value)
        logger.debug(f"u8 validation successful: {value}")
        return value
    except ValueError as e:
        logger.warning(f"u8 validation failed: {e}")
        raise ValidationError(
            message=str(e),
            field="u8_value", 
            value=value
        )


def validate_json_schema(data: Dict[str, Any], schema: Dict[str, Any]) -> Dict[str, Any]:
    """
    Validate JSON data against a schema.
    
    Args:
        data: JSON data to validate
        schema: JSON schema definition
        
    Returns:
        Validated data
        
    Raises:
        ValidationError: If validation fails
    """
    try:
        return _forzium_validation.validate_json_schema(data, schema)
    except ValueError as e:
        logger.warning(f"Schema validation failed: {e}")
        raise ValidationError(
            message=str(e),
            field="json_data"
        )


# Factory functions for validators
def buffer_validator(max_size: int = MAX_BUFFER_SIZE, strict: bool = False) -> BufferValidator:
    """Create a buffer validator with specified constraints."""
    return BufferValidator(max_size, strict)


def utf8_validator(allow_empty: bool = True, max_length: Optional[int] = None) -> Utf8Validator:
    """Create a UTF-8 validator with specified constraints."""
    return Utf8Validator(allow_empty, max_length)


def u8_validator() -> NumericValidator:
    """Create a u8 numeric validator."""
    return _forzium_validation.u8_validator()


def i32_validator(min_val: int, max_val: int) -> NumericValidator:
    """Create an i32 numeric validator with range."""
    return _forzium_validation.i32_validator(min_val, max_val)


def f64_validator(min_val: float, max_val: float) -> NumericValidator:
    """Create an f64 numeric validator with range."""
    return _forzium_validation.f64_validator(min_val, max_val)


def schema_validator(strict: bool = False) -> SchemaValidator:
    """Create a JSON schema validator."""
    return SchemaValidator(strict)

__all__ = [
    # High-level functions
    "validate_buffer_size",
    "validate_utf8_string", 
    "validate_u8_range",
    "validate_json_schema",
    # Validator classes
    "BufferValidator",
    "Utf8Validator",
    "NumericValidator", 
    "SchemaValidator",
    # Factory functions
    "buffer_validator",
    "utf8_validator",
    "u8_validator",
    "i32_validator",
    "f64_validator",
    "schema_validator",
    # Constants
    "MAX_BUFFER_SIZE",
    "U8_MIN",
    "U8_MAX",
]