# Rust Dashboard

A native cross-platform system monitoring dashboard built with Rust and [Tauri v2](https://v2.tauri.app/), featuring a [SvelteKit](https://kit.svelte.dev/) frontend with glassmorphism design. Real-time CPU, memory, disk, and network monitoring with process management, detachable panels, and a menu bar tray popup.

## Features

- **Real-Time Monitoring** - CPU (global + per-core), memory, disk, and network stats updated every 1-60 seconds
- **Process Management** - Aggregated process list with search, CPU/memory filters, sortable columns, and kill with confirmation
- **Historical Charts** - Time-series graphs for CPU and memory usage (last 300 data points) via Chart.js
- **Menu Bar Tray** - Quick-glance system stats popup from the menu bar icon, with proper multi-monitor positioning
- **Detachable Panels** - Pop out any panel (CPU, Memory, Disk, Network, Processes) into its own window
- **Dark/Light Theme** - Glassmorphism UI with theme persistence across all windows including tray popup
- **Export** - Export system snapshots to JSON or CSV with formula injection protection
- **Configuration** - Persistent settings (refresh interval, theme) via TOML config file

## Tech Stack

- **Backend**: Rust + Tauri v2 + sysinfo
- **Frontend**: SvelteKit + TypeScript + Chart.js
- **Build**: Cargo workspace + Vite + adapter-static
- **CI/CD**: GitHub Actions (multi-platform test, lint, audit, release)

## Getting Started

### Prerequisites

- Rust 1.70+ (2021 edition)
- Node.js 24+
- npm

### Installation

```bash
git clone https://github.com/Technical-1/Rust-Dashboard.git
cd Rust-Dashboard
cd ui && npm install && cd ..
```

### Development

```bash
# Start dev server (backend + frontend with hot reload)
cargo tauri dev

# Run library tests
cargo test -p rust_dashboard_lib --verbose

# Check formatting and lints
cargo fmt -- --check
cargo clippy --workspace -- -D warnings

# Build frontend only
cd ui && npm run build
```

### Production Build

```bash
cargo tauri build
```

Builds a native `.app` (macOS), `.msi` (Windows), or `.deb`/`.AppImage` (Linux).

## Architecture

```
Rust-Dashboard/
├── src/                    # Library crate (rust_dashboard_lib)
│   ├── lib.rs              # Module exports
│   ├── system.rs           # SystemMonitor - sysinfo wrapper
│   ├── config.rs           # AppConfig - TOML persistence
│   └── error.rs            # DashboardError types
├── src-tauri/              # Binary crate (Tauri v2 app)
│   ├── src/main.rs         # Tauri commands, tray, background thread
│   ├── capabilities/       # Split permissions (main vs panels)
│   ├── icons/              # App icons (gauge design)
│   └── tauri.conf.json     # Window config, CSP, build settings
├── ui/                     # SvelteKit frontend
│   ├── src/lib/components/ # Svelte components (15 total)
│   ├── src/lib/stores/     # Reactive stores (system, config, processes)
│   └── src/routes/         # SvelteKit routes (single page, multi-mode)
├── tests/                  # Integration tests
├── examples/               # Library usage example
└── .github/workflows/      # CI (test/lint/audit) + Release
```

## Using as a Library

```rust
use rust_dashboard_lib::system::SystemMonitor;

let mut monitor = SystemMonitor::new();
monitor.refresh();

let cpu = monitor.global_cpu_usage();
let (used, free, total, avail, swap_used, swap_total) = monitor.memory_info();
let processes = monitor.combined_process_list();
```

## Security

Defenses are layered across the IPC boundary, the filesystem boundary, and the dependency supply chain:

- **Path traversal prevention** — File exports walk up to the deepest existing path ancestor, canonicalize it, verify it lies under the user's home directory, then create any missing intermediate directories with a defense-in-depth re-canonicalize that catches lexical `..` escapes after `create_dir_all`.
- **PID guard at both layers** — `SystemMonitor::kill_process` in the library and the Tauri command wrapper both refuse PID 0 and PID 1, so neither a misbehaving frontend nor a third-party library consumer can request termination of the kernel or init.
- **Split Tauri capabilities** — Detached panels and the tray popup get a separate capability set without `create-webview-window` or process-kill permissions; only the main window can spawn new windows or terminate processes.
- **Strict CSP** — `default-src 'self' tauri: asset:; script-src 'self'; ...` with explicit Tauri-protocol origins on the IPC bridge and no `unsafe-inline` scripts.
- **CSV formula-injection guard** — Export quotes every field and prefixes `=`, `+`, `-`, `@`, `\t`, `\r` with a single quote so the value can't be reinterpreted as a formula on import.
- **CI security gates** — `cargo audit` and `npm audit --audit-level=moderate` must both exit 0 with no bypass flags. GitHub Actions pinned to immutable commit SHAs.

## License

MIT

## Author

Jacob Kanfer - [GitHub](https://github.com/Technical-1)
