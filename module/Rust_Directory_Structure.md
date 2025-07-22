
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