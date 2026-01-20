# Project Q&A

## Overview

**Rust-Dashboard** is a cross-platform system monitoring application built entirely in Rust. It provides real-time visualization of CPU usage, memory consumption, disk space, network activity, and running processes through a native GUI. I built this to learn Rust's GUI ecosystem and concurrency primitives while creating a practical tool I actually use daily to monitor my development machine.

### Problem Solved

I needed a lightweight system monitor that:
- Runs natively without a browser or Electron overhead
- Provides historical trends, not just point-in-time snapshots
- Aggregates processes by name (like macOS Activity Monitor)
- Works consistently across macOS, Linux, and Windows

Commercial tools like iStat Menus are macOS-only, and web-based solutions add unnecessary resource overhead for something that should be minimal.

### Target Users

- Developers who want to monitor resource usage during builds and tests
- System administrators managing local or remote machines
- Rust enthusiasts interested in GUI development patterns
- Anyone who wants a lightweight alternative to bloated system monitors

## Key Features

### Real-Time System Monitoring
Live CPU and memory usage with color-coded thresholds (green < 50%, yellow < 80%, red >= 80%). Progress bars provide at-a-glance status.

### Historical Charts
I store the last 5 minutes of CPU and memory data in VecDeque buffers, rendered as time-series charts using egui_plot. This helps identify trends and transient spikes.

### Process Management
Full process list with search, filtering by CPU/memory thresholds, and sortable columns. Processes are aggregated by name - so "Chrome" shows combined stats for all Chrome processes rather than 20 separate entries.

### Process Details & Actions
Click any process to expand and see its full command line, start time, and parent PID. Kill buttons with confirmation dialogs allow terminating runaway processes.

### Theme Support
Dark and light themes that persist across sessions. The theme toggle is in the top panel for quick switching.

### Export Functionality
Export current system state to JSON or CSV for logging, reporting, or further analysis.

### Persistent Configuration
Settings (refresh interval, theme, window position) save automatically to a platform-appropriate config directory using TOML format.

## Technical Highlights

### Challenge: UI Thread Blocking

**Problem:** System queries (especially process enumeration) can take 50-100ms on busy systems. Running these on the UI thread caused visible lag.

**Solution:** I spawn a dedicated background thread that refreshes system data on a configurable interval. Data is shared via `Arc<Mutex<SystemMonitor>>` with minimal lock duration - the UI thread copies all needed data in a single lock scope, then releases immediately before rendering.

### Challenge: Process List Performance

**Problem:** Systems can have 500+ processes. Rendering all of them every frame was slow, and the list was overwhelming to read.

**Solution:** I aggregate processes by name using a HashMap, combining CPU and memory usage across all PIDs sharing that name. The aggregation happens during refresh (not render), and results are cached. Sorting uses indices into the cached list rather than cloning process data.

### Challenge: Synchronized Refresh Intervals

**Problem:** When the user changes the refresh interval in the UI, the background thread needed to pick up the new value without race conditions or requiring a restart.

**Solution:** The refresh interval is stored in an `AtomicU32` shared between UI and background threads. The background thread reads it via `Ordering::Relaxed` after each sleep cycle. No mutex needed for this one-way communication.

### Challenge: Memory Usage Display Consistency

**Problem:** Different platforms report memory in different units, and mixing bytes/KB/MB/GB was confusing.

**Solution:** I normalize all memory values to bytes internally, then format them consistently as GiB in the UI using a shared `format_bytes()` helper.

### Innovative Approach: Immediate-Mode GUI for System Monitoring

Most system monitors use retained-mode GUI frameworks (Qt, GTK, Electron). I used egui's immediate-mode approach where the entire UI is re-rendered each frame based on current state. This eliminates widget state synchronization bugs and makes the code much simpler - there's no "update process list widget" function, just "render whatever is in `self.processes`".

## Frequently Asked Questions

### Q: Why Rust instead of Python/Go/C++?

I chose Rust because:
1. **Memory safety without garbage collection** - Important for a long-running monitoring tool
2. **Excellent cross-platform support** - Same code compiles for all three major OSes
3. **Strong typing catches bugs early** - Process PIDs, memory sizes, etc. are distinct types
4. **I wanted to learn Rust GUI development** - This project taught me egui, wgpu, and Rust's concurrency model

### Q: Why egui instead of Tauri/Electron/Qt?

egui is pure Rust with no native dependencies (uses wgpu for rendering). This means:
- Single `cargo build` produces a working binary
- No node_modules, no Qt installation, no web browser embedded
- ~10MB binary vs 100MB+ for Electron apps
- Actually uses less RAM than what it's monitoring

### Q: How accurate are the CPU percentages?

sysinfo reports CPU usage as the percentage of one core. On a 4-core system, a process using 2 full cores would show 200%. This matches how `top` reports on Linux/macOS. I display it as-is rather than normalizing because developers typically want to know "how many cores is this using?"

### Q: Why can't I see disk I/O speeds?

This is a limitation of sysinfo 0.33.1. The Disk API only provides space information (used/available/total), not read/write speeds or IOPS. I document this in the README. A future version might use platform-specific APIs to add this.

### Q: Why is there no "terminate" option, only "kill"?

sysinfo's `Process::terminate()` method was removed in recent versions. Only `Process::kill()` (SIGKILL on Unix) is available. I added confirmation dialogs to prevent accidents, but there's no graceful termination option currently.

### Q: Does this work on Wayland?

Yes, with caveats. eframe/wgpu supports Wayland, but window position persistence may not work correctly on some Wayland compositors that don't allow applications to set their own position.

### Q: How do I add this to my portfolio?

The `.portfolio/` directory contains three documentation files:
- `architecture.md` - System diagrams and design decisions
- `stack.md` - Technology choices and versions
- `qa.md` - This file

These are designed to be shown to interviewers or included in portfolio websites.

### Q: What's the test coverage like?

I have 41 tests covering:
- System monitoring functionality (basic queries, edge cases)
- Configuration persistence (load, save, defaults)
- Export functionality (JSON, CSV formatting)
- Concurrent access patterns (multiple threads refreshing)
- Error conditions (invalid PIDs, missing processes)

### Q: Can I use SystemMonitor as a library in my own project?

Yes! The `rust_dashboard_lib` crate exposes `SystemMonitor` and related types. See `examples/basic_usage.rs` for a complete example of using it without the GUI.

### Q: How do I contribute?

1. Fork the repository
2. Run `cargo test` to ensure tests pass
3. Run `cargo fmt` and `cargo clippy` before committing
4. Submit a PR with a clear description of changes

The CI pipeline will verify your changes build on all platforms.
