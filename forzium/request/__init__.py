"""Minimal Request objects used for tests."""

from dataclasses import dataclass, field
from typing import Dict, Optional


@dataclass
class Request:
    method: str
    path: str
    headers: Dict[str, str] = field(default_factory=dict)
    query_params: Dict[str, str] = field(default_factory=dict)
    body: Optional[bytes] = None
    path_params: Dict[str, str] = field(default_factory=dict)


__all__ = ["Request"]