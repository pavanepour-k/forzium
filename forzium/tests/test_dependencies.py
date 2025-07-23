import pytest
from typing import Optional, Dict, Any
from forzium.dependencies import DependencyInjector
from forzium.request import Request

# Test service classes
class UserService:
    """Mock user service for testing."""
    pass

class Database:
    """Mock database service for testing."""
    pass

class CacheService:
    """Mock cache service for testing."""
    pass

class ServiceA:
    """Service with dependency on ServiceB (for circular dependency test)."""
    def __init__(self, service_b: 'ServiceB'):
        self.service_b = service_b

class ServiceB:
    """Service with dependency on ServiceA (for circular dependency test)."""
    def __init__(self, service_a: ServiceA):
        self.service_a = service_a


class TestDependencyExtraction:
    """Test dependency extraction from function signatures."""
    
    def test_get_dependencies_basic(self):
        """Test extracting dependencies from function signature."""
        injector = DependencyInjector()

        # Sample function with dependencies
        def sample_handler(
            request: Request,
            user_service: UserService,
            db: Database,
            cache: Optional[CacheService] = None
        ) -> Dict[str, Any]:
            return {"status": "ok"}

        # Extract dependencies
        deps = injector.get_dependencies(sample_handler)

        # Verify correct dependencies extracted
        assert 'user_service' in deps
        assert deps['user_service'] == UserService
        assert 'db' in deps
        assert deps['db'] == Database
        assert 'cache' in deps
        # Note: Optional[CacheService] appears as the full type
        assert str(deps['cache']).startswith('typing.Optional')

        # Verify 'request' is excluded
        assert 'request' not in deps

    def test_get_dependencies_no_annotations(self):
        """Test function with no type annotations."""
        injector = DependencyInjector()

        def handler_no_types(request, service):
            return {"status": "ok"}

        deps = injector.get_dependencies(handler_no_types)
        
        # Should return empty dict for unannotated params
        assert len(deps) == 0

    def test_get_dependencies_mixed_annotations(self):
        """Test function with mixed annotated and unannotated parameters."""
        injector = DependencyInjector()

        def mixed_handler(
            request: Request,
            typed_service: UserService,
            untyped_param,
            another_typed: Database
        ):
            return {"status": "ok"}

        deps = injector.get_dependencies(mixed_handler)
        
        assert 'typed_service' in deps
        assert deps['typed_service'] == UserService
        assert 'another_typed' in deps
        assert deps['another_typed'] == Database
        assert 'untyped_param' not in deps
        assert 'request' not in deps

    def test_get_dependencies_class_method(self):
        """Test extracting dependencies from class methods."""
        injector = DependencyInjector()

        class Controller:
            def handle(self, request: Request, service: UserService):
                return {"status": "ok"}

        controller = Controller()
        deps = injector.get_dependencies(controller.handle)
        
        assert 'service' in deps
        assert deps['service'] == UserService
        assert 'self' not in deps
        assert 'request' not in deps


class TestCircularDependencies:
    """Test circular dependency detection."""
    
    def test_circular_dependency_detection(self):
        """Test that circular dependencies are detected and raise error."""
        injector = DependencyInjector()

        # Register services with circular dependency
        injector.register(ServiceA, lambda: ServiceA(injector.get(ServiceB)))
        injector.register(ServiceB, lambda: ServiceB(injector.get(ServiceA)))

        # Attempting to resolve should raise ValueError
        with pytest.raises(ValueError, match="Circular dependency detected"):
            injector.get(ServiceA)
            
        # Should also fail when starting from ServiceB
        with pytest.raises(ValueError, match="Circular dependency detected"):
            injector.get(ServiceB)

    def test_self_dependency_detection(self):
        """Test self-referential dependency detection."""
        injector = DependencyInjector()

        class SelfDependent:
            def __init__(self, dep: 'SelfDependent'):
                self.dep = dep

        injector.register(SelfDependent, lambda: SelfDependent(injector.get(SelfDependent)))

        with pytest.raises(ValueError, match="Circular dependency detected"):
            injector.get(SelfDependent)


class TestDependencyScopes:
    """Test different dependency scopes."""
    
    def test_singleton_scope(self):
        """Test singleton dependency scope."""
        injector = DependencyInjector()
        
        # Counter to track instantiations
        instantiation_count = 0
        
        class SingletonService:
            def __init__(self):
                nonlocal instantiation_count
                instantiation_count += 1
                self.id = instantiation_count
        
        injector.register(SingletonService, SingletonService, singleton=True)
        
        # Get instance multiple times
        instance1 = injector.get(SingletonService)
        instance2 = injector.get(SingletonService)
        instance3 = injector.get(SingletonService)
        
        # Should be same instance
        assert instance1 is instance2
        assert instance2 is instance3
        assert instantiation_count == 1
        assert instance1.id == 1

    def test_transient_scope(self):
        """Test transient dependency scope (new instance each time)."""
        injector = DependencyInjector()
        
        instantiation_count = 0
        
        class TransientService:
            def __init__(self):
                nonlocal instantiation_count
                instantiation_count += 1
                self.id = instantiation_count
        
        injector.register(TransientService, TransientService, singleton=False)
        
        # Get instance multiple times
        instance1 = injector.get(TransientService)
        instance2 = injector.get(TransientService)
        instance3 = injector.get(TransientService)
        
        # Should be different instances
        assert instance1 is not instance2
        assert instance2 is not instance3
        assert instantiation_count == 3
        assert instance1.id == 1
        assert instance2.id == 2
        assert instance3.id == 3


class TestDependencyResolution:
    """Test dependency resolution functionality."""
    
    @pytest.mark.asyncio
    async def test_resolve_all_dependencies(self):
        """Test resolving all dependencies for a handler."""
        injector = DependencyInjector()
        
        # Register test services
        injector.register(UserService, UserService, singleton=True)
        injector.register(Database, Database, singleton=True)
        injector.register(CacheService, CacheService, singleton=False)
        
        # Handler with dependencies
        async def handler(
            request: Request,
            user_service: UserService,
            db: Database,
            cache: CacheService
        ):
            return {
                "has_user_service": isinstance(user_service, UserService),
                "has_db": isinstance(db, Database),
                "has_cache": isinstance(cache, CacheService)
            }
        
        # Resolve dependencies
        resolved = await injector.resolve_all(handler)
        
        # Verify all dependencies resolved
        assert 'user_service' in resolved
        assert isinstance(resolved['user_service'], UserService)
        assert 'db' in resolved
        assert isinstance(resolved['db'], Database)
        assert 'cache' in resolved
        assert isinstance(resolved['cache'], CacheService)
        
        # Verify 'request' not included
        assert 'request' not in resolved

    def test_unregistered_dependency_error(self):
        """Test error when dependency not registered."""
        injector = DependencyInjector()
        
        # Don't register UserService
        
        with pytest.raises(ValueError, match="Dependency .* not registered"):
            injector.get(UserService)

    def test_factory_vs_instance_registration(self):
        """Test registering factory functions vs instances."""
        injector = DependencyInjector()
        
        # Register with factory function
        def create_user_service():
            service = UserService()
            service.created_by = "factory"
            return service
        
        injector.register(UserService, create_user_service)
        
        # Register with instance (acts as factory returning same instance)
        db_instance = Database()
        db_instance.created_by = "instance"
        injector.register(Database, lambda: db_instance)
        
        # Verify factory creates new instance
        user_service = injector.get(UserService)
        assert hasattr(user_service, 'created_by')
        assert user_service.created_by == "factory"
        
        # Verify instance registration returns same object
        db = injector.get(Database)
        assert db is db_instance
        assert db.created_by == "instance"


class TestCachingBehavior:
    """Test dependency injection caching behavior."""
    
    def test_get_dependencies_caching(self):
        """Test that get_dependencies result is cached."""
        injector = DependencyInjector()
        
        def handler(service: UserService, db: Database):
            pass
        
        # First call
        deps1 = injector.get_dependencies(handler)
        
        # Second call should return cached result
        deps2 = injector.get_dependencies(handler)
        
        # Should be same dict (due to lru_cache)
        assert deps1 is deps2


if __name__ == "__main__":
    pytest.main([__file__, "-v"])