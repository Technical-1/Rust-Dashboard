# Technology Stack

## Language

| Component | Version | Notes |
|-----------|---------|-------|
| Rust | 1.60+ (2021 Edition) | Primary language for the entire application |

## Core Dependencies

### GUI Framework

| Crate | Version | Purpose |
|-------|---------|---------|
| **eframe** | 0.31.1 | Window management and application lifecycle |
| **egui** | (via eframe) | Immediate-mode GUI library |
| **egui_extras** | 0.31.1 | Additional widgets (TableBuilder for process list) |
| **egui_plot** | 0.31 | Time-series charts for CPU/memory history |

I chose egui/eframe because:
- **Immediate mode** simplifies state management - no need to sync widget state with data model
- **Pure Rust** means easy cross-compilation without linking to platform GUI libraries
- **wgpu backend** provides GPU-accelerated rendering with low overhead
- The API is simple enough to iterate quickly while building features

### System Information

| Crate | Version | Purpose |
|-------|---------|---------|
| **sysinfo** | 0.33.1 | Cross-platform system metrics (CPU, memory, disk, network, processes) |

I chose sysinfo because:
- Single crate provides all system metrics I needed
- Cross-platform support (Windows, macOS, Linux)
- Active maintenance with frequent updates
- Well-documented API

### Serialization & Configuration

| Crate | Version | Purpose |
|-------|---------|---------|
| **serde** | 1.0 | Serialization framework |
| **serde_json** | 1.0 | JSON export functionality |
| **toml** | 0.8 | Configuration file parsing |
| **csv** | 1.3 | CSV export functionality |
| **dirs** | 5.0 | Platform-appropriate config directory paths |

I chose TOML for configuration because:
- Human-readable and editable
- Native support for Rust data types
- Comments are supported (unlike JSON)
- Simple syntax for flat configuration structures

### Error Handling & Logging

| Crate | Version | Purpose |
|-------|---------|---------|
| **thiserror** | 1.0 | Derive macro for custom error types |
| **log** | 0.4 | Logging facade |
| **env_logger** | 0.11 | Runtime-configurable logging implementation |

I chose thiserror over anyhow because:
- Library code benefits from structured error types
- Callers can match on specific error variants
- Cleaner error messages in the UI

## Build Configuration

### Release Profile

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = "fat"          # Full link-time optimization
codegen-units = 1    # Single codegen unit for best optimization
strip = true         # Strip symbols from binary
panic = "abort"      # Smaller binaries
```

I use aggressive optimization because:
- Dashboard runs continuously; CPU efficiency matters
- Smaller binary size (strip + panic=abort)
- LTO enables cross-crate inlining, critical for egui performance

### Dev Profile

```toml
[profile.dev]
opt-level = 1        # Some optimization for better dev experience
```

I set opt-level=1 in dev mode because:
- System monitoring still needs reasonable performance during development
- Full debug builds are too slow for UI work

## Infrastructure

### CI/CD Pipeline (GitHub Actions)

| Job | Platforms | Purpose |
|-----|-----------|---------|
| Test | ubuntu, macos, windows | Cross-platform test execution |
| Lint | ubuntu | Clippy lints + format check |

The CI pipeline runs on all three major platforms because:
- sysinfo has platform-specific code paths
- egui rendering can behave differently per platform
- Ensures the dashboard builds for all target users

### Platform-Specific Notes

**Linux:**
- Requires GTK3 and X11 development libraries
- `libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`

**macOS:**
- No special dependencies (uses native frameworks)
- Code signing setup documented in `MACOS_SIGNING_SETUP.md`

**Windows:**
- No special dependencies
- Uses native Windows rendering

## Project Structure

```
Rust-Dashboard/
├── src/
│   ├── main.rs      # Binary: GUI application
│   ├── lib.rs       # Library: module exports
│   ├── system.rs    # SystemMonitor implementation
│   ├── config.rs    # Configuration persistence
│   └── error.rs     # Custom error types
├── examples/
│   └── basic_usage.rs
├── tests/
│   ├── test_system.rs
│   ├── test_config.rs
│   ├── test_export.rs
│   └── test_process_actions.rs
└── .github/workflows/
    ├── ci.yml       # Test + lint
    └── release.yml  # Release automation
```

## Dependency Philosophy

I follow these principles when choosing dependencies:

1. **Prefer well-maintained crates** - All dependencies have recent commits and active maintainers
2. **Minimize dependency tree** - Each crate must justify its inclusion
3. **Use version ranges conservatively** - Major versions pinned, minor versions allowed to float
4. **Audit for security** - No dependencies flagged by `cargo audit`

## Version Constraints

The project uses Rust 2021 edition and requires Rust 1.60+ due to:
- `let-else` patterns used in error handling
- Disjoint capture in closures
- IntoIterator for arrays

## Future Considerations

Dependencies I may add in future versions:
- **rfd** - Native file dialogs for export functionality
- **notify** - File system watching for configuration hot-reload
- **tokio** - If async operations become necessary

Dependencies I explicitly avoid:
- Heavy async runtimes (not needed for this use case)
- Full web frameworks (this is a desktop app)
- Native GUI toolkits (prefer pure Rust portability)
