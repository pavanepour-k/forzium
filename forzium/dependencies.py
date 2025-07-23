# Lightweight dependency injection container for Forzium

from __future__ import annotations

import inspect
from functools import lru_cache
from typing import Any, Callable, Dict, Type


class DependencyInjector:
    """Simple dependency injection container."""

    def __init__(self) -> None:
        self._registry: Dict[Type[Any], tuple[Callable[[], Any], bool]] = {}
        self._singletons: Dict[Type[Any], Any] = {}
        self._stack: list[Type[Any]] = []

    def register(self, key: Type[Any], factory: Callable[[], Any], *, singleton: bool = True) -> None:
        """Register a dependency factory.

        Parameters
        ----------
        key:
            Type used as lookup key.
        factory:
            Callable returning the dependency instance.
        singleton:
            Whether the same instance is returned on each call.
        """
        self._registry[key] = (factory, singleton)

    def get(self, key: Type[Any]) -> Any:
        """Retrieve an instance for the given key."""
        if key not in self._registry:
            raise ValueError(f"Dependency {key} not registered")

        if key in self._singletons:
            return self._singletons[key]

        if key in self._stack:
            raise ValueError("Circular dependency detected")

        factory, singleton = self._registry[key]
        self._stack.append(key)
        try:
            instance = factory()
        finally:
            self._stack.pop()

        if singleton:
            self._singletons[key] = instance
        return instance

    @staticmethod
    @lru_cache(maxsize=None)
    def _analyze(func: Callable[..., Any]) -> Dict[str, Any]:
        sig = inspect.signature(func)
        deps: Dict[str, Any] = {}
        for name, param in sig.parameters.items():
            if name in {"self", "request"}:
                continue
            if param.annotation is not inspect._empty:
                deps[name] = param.annotation
        return deps

    def get_dependencies(self, func: Callable[..., Any]) -> Dict[str, Any]:
        """Extract dependency annotations from a callable."""
        return self._analyze(func)

    async def resolve_all(self, func: Callable[..., Any]) -> Dict[str, Any]:
        """Resolve all dependencies for ``func`` asynchronously."""
        deps = {}
        for name, dep_type in self.get_dependencies(func).items():
            value = self.get(dep_type)
            if inspect.isawaitable(value):
                value = await value
            deps[name] = value
        return deps


__all__ = ["DependencyInjector"]