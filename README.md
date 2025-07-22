# Forzium_Directory_Structure.md
 - Directory organization for performance-critical Python applications with Rust extensions

```md
project-root/module/ # (RUST CRATES)
├── forzium-core/ # Pure Rust logic (was: core/)
│	├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── validation/
│   │   ├── routing/
│   │   ├── dependencies/
│   │   ├── request/
│   │   └── response/
│   └── tests/
├── pyo3-forzium/ # PyO3 bindings (was: bindings/)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs # Python module: _forzium
│   │   ├── interface/ # Python interface layer
│ 	│   │   ├── mod.rs
│ 	│   │   └── config_bridge.rs
│   │   └── ...
│   └── pyproject.toml
└── Cargo.toml # Workspace root
```
## Python Directory Structure

```md
project-root/forzium/
├── __init__.py
├── _module.py # Import from _forzium
├── validation/
├── routing/
├── dependencies/
├── request/
├── response/
└── tests/
```

## Share Resources Directory Structure

```md
project-root/
└── shared/ # Shared resources (was: share/)
    ├── proto/
    ├── schemas/
    └── constants/
```

## Documents Directory Structure

```md
project-root/
│
├── .github/
│   └── workflows/
│       ├── rust.yml # RUST CI
│       ├── python.yml # PYTHON CI
│       └── integration.yml # CROSS-LANGUAGE CI
├── requirements.txt # Python dependencies
├── pyproject.toml # Python project configuration
├── Cargo.toml # Workspace-level Rust configuration
└── README.md # Project documentation
```