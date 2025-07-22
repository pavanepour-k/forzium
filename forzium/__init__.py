"""
FastAPI to Forzium Migration Example

This example demonstrates how to migrate a FastAPI application to use
Forzium's Rust-powered components while maintaining API compatibility.
"""

import asyncio
from typing import Dict, List, Optional
from datetime import datetime

# FastAPI Original Code (commented for comparison)
"""
from fastapi import FastAPI, HTTPException, Depends, Query
from pydantic import BaseModel, validator

app = FastAPI()

class User(BaseModel):
    id: int
    username: str
    email: str
    
    @validator('email')
    def email_validation(cls, v):
        if '@' not in v:
            raise ValueError('Invalid email')
        return v

async def get_current_user(token: str = Query(...)):
    # Simulate user lookup
    if token != "valid_token":
        raise HTTPException(status_code=401, detail="Invalid token")
    return User(id=1, username="john", email="john@example.com")

@app.get("/users/me")
async def read_current_user(current_user: User = Depends(get_current_user)):
    return current_user

@app.post("/users")
async def create_user(user: User):
    # Validate and create user
    return {"message": f"User {user.username} created"}
"""

# Forzium Implementation
import forzium
from forzium import (
    RequestHandler,
    Request,
    Response,
    DependencyInjector,
    ValidationError,
    ok,
    bad_request,
    unauthorized,
    created,
)
from forzium.validators import validate_utf8_string, SchemaValidator


# User model with Rust-powered validation
class User:
    def __init__(self, id: int, username: str, email: str):
        self.id = id
        self.username = username
        self.email = email
        
        # Validate email using Rust
        if '@' not in email:
            raise ValidationError("Invalid email", field="email", value=email)
    
    def to_dict(self) -> Dict[str, any]:
        return {
            "id": self.id,
            "username": self.username,
            "email": self.email
        }
    
    @classmethod
    def from_json(cls, data: Dict[str, any]) -> 'User':
        """Create User from JSON data with validation."""
        # Use Rust schema validator
        validator = SchemaValidator()
        validator.require_field("id", "number")
        validator.require_field("username", "string")
        validator.require_field("email", "string")
        
        validated_data = validator.validate(data)
        return cls(**validated_data)


# Dependency injection setup
injector = DependencyInjector()


# Current user dependency
class UserService:
    """Service for user operations."""
    
    async def get_current_user(self, token: str) -> User:
        """Simulate user lookup based on token."""
        if token != "valid_token":
            raise ValidationError("Invalid token", field="token")
        
        return User(id=1, username="john", email="john@example.com")


# Register service as singleton
injector.register(UserService, UserService, singleton=True)


# Create application
app = RequestHandler()


# Helper to extract token from request
async def get_token(request: Request) -> str:
    """Extract token from query params or headers."""
    # Check query params first
    token = request.query_params.get("token")
    if token:
        return token
    
    # Check Authorization header
    auth_header = request.get_header("Authorization")
    if auth_header and auth_header.startswith("Bearer "):
        return auth_header[7:]
    
    raise ValidationError("Token required", field="token")


# Routes
@app.route("/users/me", methods=["GET"])
async def read_current_user(request: Request, user_service: UserService):
    """Get current user information."""
    try:
        token = await get_token(request)
        user = await user_service.get_current_user(token)
        return ok(user.to_dict())
    except ValidationError as e:
        if e.field == "token":
            return unauthorized("Invalid or missing token")
        return bad_request(str(e))


@app.route("/users", methods=["POST"])
async def create_user(request: Request):
    """Create a new user."""
    try:
        # Parse JSON body using Rust
        user_data = request.json()
        if not user_data:
            return bad_request("Request body required")
        
        # Validate and create user
        user = User.from_json(user_data)
        
        # In real app, save to database here
        
        return created(
            {"message": f"User {user.username} created", "user": user.to_dict()},
            location=f"/users/{user.id}"
        )
    except ValidationError as e:
        return bad_request(f"Validation error: {e}")
    except Exception as e:
        return bad_request(f"Invalid request: {e}")


# Advanced example with Rust validators
@app.route("/validate", methods=["POST"])
async def validate_data(request: Request):
    """Demonstrate various Rust validators."""
    try:
        data = request.json()
        
        # Buffer size validation
        if "file_data" in data:
            file_bytes = data["file_data"].encode()
            forzium.validate_buffer_size(file_bytes)
        
        # UTF-8 validation
        if "text_data" in data:
            text_bytes = data["text_data"].encode()
            validated_text = validate_utf8_string(text_bytes)
        
        # Numeric range validation
        if "age" in data:
            age = data["age"]
            if not 0 <= age <= 150:
                raise ValidationError("Age out of range", field="age", value=age)
        
        # Schema validation
        if "user_data" in data:
            user_validator = SchemaValidator()
            user_validator.require_field("name", "string")
            user_validator.require_field("email", "string")
            user_validator.optional_field("phone", "string")
            
            validated_user = user_validator.validate(data["user_data"])
        
        return ok({"message": "All validations passed"})
        
    except ValidationError as e:
        return bad_request({"error": str(e), "field": e.field})
    except Exception as e:
        return bad_request({"error": str(e)})


# Performance comparison endpoint
@app.route("/benchmark/json", methods=["POST"])
async def benchmark_json_parsing(request: Request):
    """Benchmark Rust JSON parsing performance."""
    import time
    
    # Rust parsing (via request.json())
    start = time.perf_counter()
    parsed_data = request.json()
    rust_time = time.perf_counter() - start
    
    # Python parsing for comparison
    import json as py_json
    start = time.perf_counter()
    py_parsed = py_json.loads(request.body.decode())
    python_time = time.perf_counter() - start
    
    return ok({
        "rust_parse_time": f"{rust_time*1000:.3f}ms",
        "python_parse_time": f"{python_time*1000:.3f}ms",
        "speedup": f"{python_time/rust_time:.2f}x",
        "data_size": len(request.body),
    })


# Example middleware
async def timing_middleware(request: Request) -> Request:
    """Add request timing information."""
    request.start_time = datetime.now()
    return request


async def cors_middleware(request: Request) -> Request:
    """Add CORS headers to responses."""
    # In real implementation, would modify response
    return request


# Add middleware
app.add_middleware(timing_middleware)
app.add_middleware(cors_middleware)


# Example of running the app (for testing)
async def test_app():
    """Test the application with sample requests."""
    
    # Test GET /users/me with valid token
    request = Request(
        method="GET",
        path="/users/me",
        query_params={"token": "valid_token"}
    )
    response = await app.handle_request(request)
    print(f"GET /users/me: {response}")
    
    # Test POST /users
    user_data = {
        "id": 2,
        "username": "jane",
        "email": "jane@example.com"
    }
    request = Request(
        method="POST",
        path="/users",
        body=forzium.json.dumps(user_data).encode(),
        headers={"Content-Type": "application/json"}
    )
    response = await app.handle_request(request)
    print(f"POST /users: {response}")
    
    # Test validation endpoint
    validation_data = {
        "text_data": "Hello, 世界!",
        "age": 25,
        "user_data": {
            "name": "Test User",
            "email": "test@example.com"
        }
    }
    request = Request(
        method="POST",
        path="/validate",
        body=forzium.json.dumps(validation_data).encode(),
        headers={"Content-Type": "application/json"}
    )
    response = await app.handle_request(request)
    print(f"POST /validate: {response}")


if __name__ == "__main__":
    # Run test
    asyncio.run(test_app())
    
    # For production, integrate with ASGI server:
    # uvicorn.run(app, host="0.0.0.0", port=8000)