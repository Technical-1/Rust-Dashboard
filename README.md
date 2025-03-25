# Rust-Dashboard

Rust-Dashboard is a cross-platform system monitoring dashboard built with [sysinfo](https://crates.io/crates/sysinfo) and [eframe/egui](https://github.com/emilk/egui).

## Features
- Displays current CPU usage and memory usage in GiB
- Collects and shows disk usage and network usage (per interface)
- Lists top 5 processes by CPU and memory usage
- Supports an adjustable refresh interval
- Spawns a background thread to continuously refresh system data

## Architecture
- **system.rs**: Contains the `SystemMonitor` struct which wraps `sysinfo::System`, plus extra methods for processing data (like combined processes).
- **main.rs**: The main entry point uses `RustDashboardApp`, which:
  - Spawns a background thread to refresh the system info
  - Renders a GUI using `eframe/egui`

## How It Works
1. The main application spawns a background thread on startup.
2. This thread periodically locks the `SystemMonitor` and calls `refresh()`.
3. The user interface is rendered in the main thread using egui, reading the latest data from the `SystemMonitor`.
4. Key stats are displayed:
   - CPU usage
   - Memory usage (used, free, total, available, swap)
   - Per-disk usage in GiB
   - Network usage for interfaces in use
   - Top processes by CPU and memory usage
5. The refresh interval for the background thread is set to 5 seconds by default, but the user can adjust the display refresh interval (which also triggers the UI to update).

## Requirements
- Rust 1.60+ (2023 edition recommended)
- eframe = "0.31.1"
- sysinfo = "0.33.1"

## Usage
1. Clone or download this repository.
2. In the Rust-Dashboard directory, run:
   ```bash
   cargo run
   ```
   This will compile and launch the dashboard.
3. Adjust the refresh interval from the top bar to change how frequently the UI fetches new data and repaints.