# Rust-Dashboard

A comprehensive cross-platform system monitoring dashboard built with [sysinfo](https://crates.io/crates/sysinfo) and [eframe/egui](https://github.com/emilk/egui). Features real-time system statistics, historical data visualization, process management, and extensive customization options.

## Features

### Core Monitoring
- **CPU Monitoring**: Global CPU usage with per-core breakdown
- **Memory Monitoring**: Detailed memory statistics (used, free, total, available, swap) with visual progress bars
- **Disk Monitoring**: Per-disk usage statistics with mount points and file systems
- **Network Monitoring**: Per-interface network statistics with human-readable formatting (KB/MB/GB)
- **Process Management**: Comprehensive process list with search, filter, and management capabilities

### Advanced Features
- **Historical Data & Charts**: Time-series visualization of CPU and memory usage trends using `egui_plot`
- **Process Search & Filter**: Real-time filtering by name, CPU threshold, and memory threshold
- **Process Details**: Expandable process information showing command line, start time, parent PID
- **Process Actions**: Kill/terminate processes with confirmation dialogs
- **Per-CPU Display**: Individual CPU core usage percentages with progress bars
- **Color-Coded Thresholds**: Visual indicators (🟢🟡🔴) for CPU and memory usage levels
- **Theme Support**: Dark and light themes with persistence
- **Window Persistence**: Saves and restores window size and position
- **Export Functionality**: Export system statistics to JSON or CSV format
- **Configuration File**: Persistent settings via `config.toml` file
- **Manual Refresh**: On-demand refresh button with last refresh timestamp
- **Sortable Process Tables**: Click column headers to sort processes by name, CPU%, or memory

### Performance Optimizations
- Conditional repainting (only when data changes)
- Optimized process list sorting (no unnecessary cloning)
- Minimized mutex lock duration for better concurrency
- Synchronized refresh intervals between UI and background thread

## Architecture

### Core Components
- **`src/system.rs`**: Contains the `SystemMonitor` struct which wraps `sysinfo::System`, providing methods for querying CPU, memory, disk, network, and process information
- **`src/main.rs`**: The main application using `RustDashboardApp`, which:
  - Spawns a background thread to refresh system data
  - Renders a GUI using `eframe/egui`
  - Manages UI state, historical data, and user interactions
- **`src/config.rs`**: Configuration management module for persistent settings
- **`src/error.rs`**: Custom error types using `thiserror` for better error handling
- **`src/lib.rs`**: Library exports for using `SystemMonitor` as a library

### Data Flow
1. Background thread periodically refreshes `SystemMonitor` data
2. UI thread reads latest data from `SystemMonitor` (with minimal lock duration)
3. Historical data stored in `VecDeque` for chart visualization
4. User interactions (search, filter, sort) applied to process lists
5. Configuration persisted to `config.toml` on changes

## Requirements

- Rust 1.60+ (2024 edition)
- Dependencies (automatically managed by Cargo):
  - `sysinfo = "0.33.1"`
  - `eframe = "0.31.1"` (with wgpu feature)
  - `log = "0.4"` and `env_logger = "0.11"` for logging
  - `thiserror = "1.0"` for error handling
  - `egui_plot = "0.31"` for charts
  - `serde = "1.0"` and `serde_json = "1.0"` for export
  - `csv = "1.3"` for CSV export
  - `toml = "0.8"` and `dirs = "5.0"` for configuration

## Installation

1. Clone or download this repository:
   ```bash
   git clone <repository-url>
   cd Rust-Dashboard
   ```

2. Build and run:
   ```bash
   cargo run
   ```
   This will compile and launch the dashboard.

3. For release build (optimized):
   ```bash
   cargo build --release
   ./target/release/Rust-Dashboard
   ```

## Usage

### Basic Usage

The dashboard automatically starts monitoring your system. Use the controls in the top panel:

- **Refresh Interval**: Select how often data refreshes (1s, 2s, 5s, 10s, 15s, 30s)
- **Manual Refresh**: Click the 🔄 button to refresh immediately
- **Theme Toggle**: Switch between dark (🌙) and light (☀️) themes
- **Last Refresh**: See when data was last updated

### Process Management

1. **Search Processes**: Type in the search box to filter by process name
2. **Filter by Threshold**: Use sliders to filter by CPU% or Memory (MB)
3. **View Details**: Click on a process name to expand and see details (command, start time, parent PID)
4. **Sort Processes**: Click column headers (Name, CPU%, Memory MB) to sort
5. **Kill Process**: Click "Kill" button and confirm in the dialog

### Exporting Data

Click "📥 Export to JSON" or "📥 Export to CSV" to export current system statistics. Data includes:
- CPU usage
- Memory statistics
- Process list with CPU and memory usage
- Timestamp

### Configuration

Settings are automatically saved to `config.toml` in your platform's config directory:
- **macOS**: `~/Library/Application Support/rust-dashboard/config.toml`
- **Linux**: `~/.config/rust-dashboard/config.toml`
- **Windows**: `%APPDATA%\rust-dashboard\config.toml`

Settings include:
- Refresh interval
- Theme preference
- Window size and position

### Logging

Control log verbosity using the `RUST_LOG` environment variable:

```bash
# Debug level (most verbose)
RUST_LOG=debug cargo run

# Info level (default)
RUST_LOG=info cargo run

# Warning and errors only
RUST_LOG=warn cargo run
```

## Using as a Library

The `SystemMonitor` can be used as a library:

```rust
use rust_dashboard_lib::system::SystemMonitor;

let mut monitor = SystemMonitor::new();
monitor.refresh();

let cpu_usage = monitor.global_cpu_usage();
let (used, free, total, avail, swap_used, swap_total) = monitor.memory_info();
let processes = monitor.combined_process_list();
```

See `examples/basic_usage.rs` for a complete example.

## Running Examples

```bash
# Run the basic usage example
cargo run --example basic_usage
```

## Testing

Run all tests:

```bash
cargo test
```

Test coverage includes:
- System monitoring functionality
- Configuration management
- Export functionality
- Process management
- Concurrent access patterns
- Edge cases and error conditions

## Building

### Development Build
```bash
cargo build
```

### Release Build (Optimized)
```bash
cargo build --release
```

Release builds include:
- Maximum optimization (`opt-level = 3`)
- Link-time optimization (LTO)
- Stripped binaries for smaller size

## CI/CD

The project includes GitHub Actions CI/CD pipeline (`.github/workflows/ci.yml`) that:
- Runs tests on push/PR
- Builds for Linux, macOS, and Windows
- Checks code formatting
- Runs clippy lints

## Project Structure

```
Rust-Dashboard/
├── src/
│   ├── main.rs          # Main application and UI
│   ├── lib.rs           # Library exports
│   ├── system.rs        # System monitoring logic
│   ├── config.rs        # Configuration management
│   └── error.rs         # Custom error types
├── examples/
│   └── basic_usage.rs   # Example usage
├── tests/
│   ├── test_system.rs   # System tests
│   ├── test_config.rs   # Config tests
│   ├── test_export.rs   # Export tests
│   └── test_process_actions.rs  # Process action tests
├── .github/
│   └── workflows/
│       └── ci.yml       # CI/CD pipeline
├── Cargo.toml           # Dependencies and build config
└── README.md            # This file
```

## Key Improvements (v1.1.0)

### Code Quality
- ✅ Proper logging system replacing debug prints
- ✅ Comprehensive error handling with custom error types
- ✅ Synchronized refresh intervals between UI and background thread
- ✅ Consistent memory unit display (GiB)

### Performance
- ✅ Optimized process list sorting (no cloning)
- ✅ Conditional repainting (only when data changes)
- ✅ Minimized mutex lock duration

### Features
- ✅ Historical data visualization with charts
- ✅ Process search and filtering
- ✅ Human-readable network statistics
- ✅ Expandable process details
- ✅ Process kill/terminate with confirmation
- ✅ Dark/light theme support
- ✅ Window persistence
- ✅ JSON/CSV export
- ✅ Per-CPU usage display
- ✅ Color-coded thresholds
- ✅ Configuration file support

### Testing
- ✅ Expanded test coverage (41 tests)
- ✅ Tests for error conditions and edge cases
- ✅ Concurrent access tests
- ✅ Integration tests

### UI/UX
- ✅ Grid layout for better organization
- ✅ Visual progress bars
- ✅ Sortable process tables
- ✅ Manual refresh with timestamp
- ✅ Visual health status indicators

## Limitations

- **Disk I/O Statistics**: Not available in `sysinfo` 0.33.1. The Disk API only provides space information, not read/write speeds or I/O operations per second.
- **Process Termination**: The `terminate()` method is not available in sysinfo. Only `kill()` (SIGKILL) is supported.

## Contributing

Contributions are welcome! Please ensure:
- Code follows Rust style guidelines
- All tests pass (`cargo test`)
- Code is formatted (`cargo fmt`)
- No clippy warnings (`cargo clippy`)

## License

[Add your license here]

## Acknowledgments

- Built with [egui](https://github.com/emilk/egui) for the UI
- Uses [sysinfo](https://crates.io/crates/sysinfo) for system information
- Charts powered by [egui_plot](https://github.com/emilk/egui/tree/master/crates/egui_plot)
