"""Simple exception types used in tests."""
from typing import Any, Optional


class ProjectError(Exception):
    pass


class ValidationError(ProjectError):
    def __init__(self, message: str, field: Optional[str] = None, value: Any = None) -> None:
        super().__init__(message)
        self.field = field
        self.value = value


class ProcessingError(ProjectError):
    pass


class TimeoutError(ProjectError):
    pass


class SystemError(ProjectError):
    pass

__all__ = [
    "ProjectError",
    "ValidationError",
    "ProcessingError",
    "TimeoutError",
    "SystemError",
]