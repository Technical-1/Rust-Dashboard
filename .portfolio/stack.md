# Technology Stack

## Core Technologies

| Category | Technology | Version | Purpose |
|----------|------------|---------|---------|
| Language | Rust | 1.70+ (2021 Edition) | Backend system monitoring, Tauri commands |
| Framework | Tauri v2 | 2.x | Native desktop shell, IPC bridge, tray icon |
| Frontend | SvelteKit | 2.x | Reactive UI with routing and stores |
| UI Language | TypeScript | 5.x | Type-safe frontend code |
| Charts | Chart.js | 4.4+ | Time-series CPU and memory history graphs |
| Build | Vite | 6.x | Frontend dev server and bundler |

## Backend (Rust)

### Tauri Binary (`src-tauri/`)

| Crate | Version | Purpose |
|-------|---------|---------|
| **tauri** | 2.x | Application framework, window management, tray icon |
| **tauri-plugin-dialog** | 2.x | Native file save dialogs for export |
| **tauri-plugin-os** | 2.x | OS detection for platform-specific behavior |
| **tauri-plugin-window-state** | 2.x | Persist window position/size across restarts |
| **tauri-build** | 2.x | Build script for Tauri context generation |

I chose Tauri v2 because:
- Native performance with web UI flexibility
- ~50MB binary (vs 100MB+ Electron)
- Built-in tray icon, multi-window, and capability-based security
- Rust backend with full system access

### Library Crate (`rust_dashboard_lib`)

| Crate | Version | Purpose |
|-------|---------|---------|
| **sysinfo** | 0.33.1 | Cross-platform system metrics (CPU, memory, disk, network, processes) |
| **serde** | 1.0 | Serialization framework for IPC and config |
| **serde_json** | 1.0 | JSON export functionality |
| **csv** | 1.3 | CSV export with formula injection protection |
| **toml** | 0.8 | Configuration file parsing |
| **dirs** | 5.0 | Platform-appropriate config directory paths |
| **thiserror** | 1.0 | Derive macro for custom error types |
| **log** + **env_logger** | 0.4 / 0.11 | Structured logging with runtime-configurable levels |

I chose sysinfo because:
- Single crate for all system metrics
- Cross-platform (Windows, macOS, Linux)
- Active maintenance with frequent updates

## Frontend (SvelteKit + TypeScript)

| Package | Version | Purpose |
|---------|---------|---------|
| **@sveltejs/kit** | ^2.0.0 | Application framework with routing |
| **@sveltejs/adapter-static** | ^3.0.0 | Static site generation for Tauri embedding |
| **svelte** | ^5.0.0 | Reactive UI components |
| **@tauri-apps/api** | ^2.0.0 | IPC invoke/listen from frontend |
| **@tauri-apps/plugin-dialog** | ^2.0.0 | File save dialog bindings |
| **@tauri-apps/plugin-os** | ^2.0.0 | OS detection bindings |
| **@tauri-apps/plugin-fs** | ^2.0.0 | File system access bindings |
| **chart.js** | ^4.4.0 | Time-series charts for CPU/memory history |
| **vite** | ^6.0.0 | Dev server with HMR + production bundler |
| **typescript** | ^5.0.0 | Type safety across the frontend |

I chose SvelteKit because:
- Reactive stores map naturally to real-time data streams
- Minimal boilerplate compared to React/Vue
- Adapter-static generates a static build perfect for Tauri embedding
- Component-scoped CSS keeps styling manageable

## Build Configuration

### Release Profile (Rust)

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = "fat"          # Full link-time optimization
codegen-units = 1    # Single codegen unit for best optimization
strip = true         # Strip symbols from binary
panic = "abort"      # Smaller binaries
```

Aggressive optimization because:
- Dashboard runs continuously; CPU efficiency matters
- LTO enables cross-crate inlining for Tauri + sysinfo
- Strip + panic=abort minimizes binary size

### Dev Profile

```toml
[profile.dev]
opt-level = 1        # Some optimization for usable dev experience
```

## Infrastructure

### CI/CD (GitHub Actions)

| Workflow | Platforms | Purpose |
|----------|-----------|---------|
| **ci.yml** | ubuntu, macos, windows | Test, lint (clippy), format check, `cargo audit`, `npm audit` |
| **release.yml** | ubuntu, macos, windows | Build native installers (.app, .msi, .deb/.AppImage) |
| **rust.yml** | ubuntu, macos, windows | Library-only test matrix |

All workflows pin GitHub Actions to immutable commit SHAs for supply chain security.

### Platform Dependencies

**Linux:** `libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
**macOS:** No special dependencies (uses native WebKit)
**Windows:** No special dependencies (uses native WebView2)

## Project Structure

```
Rust-Dashboard/
├── src/                    # Library crate (rust_dashboard_lib)
│   ├── lib.rs              # Module exports
│   ├── system.rs           # SystemMonitor - sysinfo wrapper
│   ├── config.rs           # AppConfig - TOML persistence
│   └── error.rs            # DashboardError types
├── src-tauri/              # Binary crate (Tauri v2 app)
│   ├── src/main.rs         # 13 Tauri commands, tray, background thread
│   ├── capabilities/       # Split permissions (main vs panels)
│   ├── icons/              # App icons (gauge design, all sizes)
│   └── tauri.conf.json     # Window config, CSP, build settings
├── ui/                     # SvelteKit frontend
│   ├── src/lib/components/ # 15 Svelte components
│   ├── src/lib/stores/     # Reactive stores (system, config, processes)
│   ├── src/lib/types.ts    # TypeScript type definitions
│   ├── src/lib/utils.ts    # Formatting helpers
│   └── src/routes/         # SvelteKit routes (single page, multi-mode)
├── tests/                  # Integration tests (library crate)
├── examples/               # Library usage example
└── .github/workflows/      # CI (test/lint/audit) + Release
```

## Dependency Philosophy

1. **Separate concerns** — System monitoring in a reusable library, UI in a Tauri binary
2. **Minimize frontend dependencies** — No state management library (Svelte stores suffice), no CSS framework (custom glassmorphism)
3. **Pin for security** — GitHub Actions use commit SHAs, `cargo audit` + `npm audit` in CI
4. **Platform-native** — Tauri uses each OS's native webview, not a bundled browser
