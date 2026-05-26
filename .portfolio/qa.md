# Project Q&A

## Overview

**Rust Dashboard** is a native cross-platform system monitoring dashboard built with Rust and Tauri v2, featuring a SvelteKit frontend with glassmorphism design. It provides real-time CPU, memory, disk, and network monitoring with process management, detachable panels, historical charts, and a menu bar tray popup. I built this to create a lightweight, polished system monitor that leverages Rust for performance-critical system queries while using modern web technologies for a rich UI.

### Problem Solved

I needed a system monitor that:
- Runs natively without Electron's resource overhead
- Provides historical trends, not just point-in-time snapshots
- Aggregates processes by name (like macOS Activity Monitor)
- Has a menu bar tray popup for quick-glance stats
- Supports detachable panels for multi-monitor workflows
- Works consistently across macOS, Linux, and Windows

### Target Users

- Developers who want to monitor resource usage during builds and tests
- Power users who want a menu bar system monitor with detachable panels
- Rust enthusiasts interested in Tauri v2 desktop app patterns

## Key Features

### Real-Time System Monitoring
Live CPU (global + per-core), memory, disk, and network stats with color-coded thresholds (green < 50%, yellow < 80%, red >= 80%). Configurable refresh interval from 1 to 60 seconds via the TopBar dropdown.

### Historical Charts
Time-series graphs for CPU and memory usage (last 300 data points) rendered with Chart.js. Helps identify trends and transient spikes over the last ~10 minutes.

### Process Management
Aggregated process list with search, CPU/memory threshold filters, and sortable columns. Click any process to see full details (command line, start time, parent PID). Kill with confirmation dialog and PID guard (rejects PID 0/1).

### Menu Bar Tray Popup
Quick-glance system stats from the menu bar icon. Shows CPU, memory, disk, network, load average, uptime, and top 5 processes. DPI-aware positioning that works correctly across multiple monitors with different scale factors.

### Detachable Panels
Pop out any panel (CPU, Memory, Disk, Network, Processes) into its own window. Each detached panel receives the same real-time broadcast events as the main window.

### Dark/Light Theme
Glassmorphism UI with theme toggle in the TopBar. Theme persists to config file and syncs across all windows including the tray popup.

### Export
Export system snapshots to JSON or CSV. Includes path traversal prevention (canonical path allowlisting to home directory) and CSV formula injection protection.

## Technical Highlights

### Challenge: Multi-Monitor Tray Popup Positioning

**Problem:** The tray popup appeared at wrong positions on multi-monitor setups. macOS reports tray icon coordinates in physical pixels, but window positioning uses logical coordinates. Mixed-DPI setups (e.g., Retina + external monitor) caused the popup to appear on the wrong monitor entirely.

**Solution:** I use `available_monitors()` to find the monitor containing the tray icon by checking if the icon's physical coordinates fall within each monitor's bounds. I extract that monitor's scale factor, convert all coordinates to logical space, then position the popup centered under the icon. The popup is destroyed and recreated on each click (rather than show/hide) to avoid stale position state when switching monitors.

### Challenge: Thread-Safe System Monitoring

**Problem:** System queries (especially process enumeration) can take 50-100ms. Running these on the UI thread would block rendering.

**Solution:** A dedicated background thread refreshes `SystemMonitor` on a configurable interval. Data is shared via `Arc<Mutex<SystemMonitor>>` with minimal lock duration — the thread locks, refreshes, builds a `SystemSnapshot`, releases the lock, then emits the snapshot as a Tauri event. The frontend receives updates reactively through Svelte stores listening to the event bus.

### Challenge: Multi-Window State Synchronization

**Problem:** Each Tauri window (main, detached panels, tray popup) has its own SvelteKit instance with independent Svelte stores. How to keep all windows showing the same data?

**Solution:** The background thread uses `app_handle.emit("system-update", &snapshot)` which broadcasts to ALL windows simultaneously. Each window's store independently listens for this event and updates reactively. No per-window polling or state synchronization code needed.

### Challenge: Tray Popup Theme Sync

**Problem:** The tray popup is created as a separate window, so it loads with its own fresh SvelteKit instance. The theme store defaults to dark mode, ignoring the user's saved preference.

**Solution:** The `TrayPopup` component calls `loadConfig()` on mount, which reads the persisted TOML config via a Tauri command and applies the saved theme before any rendering occurs. The `toggleTheme()` function in TopBar now calls `saveCurrentConfig()` to persist immediately, so newly created tray popups always read the latest theme.

### Innovative: Glassmorphism Design System

The UI uses a custom glassmorphism design system with CSS custom properties (design tokens) for consistent styling. Semi-transparent panels with blur effects, glass borders, and color-coded status indicators create a modern, native-feeling interface without any CSS framework dependency.

### Challenge: Responsive Controls Under Long Refresh Intervals

**Problem:** The dashboard supports a 1–60 s refresh interval, surfaced through a TopBar dropdown. With a single `thread::sleep(interval_secs)` at the bottom of the monitoring loop, a user changing the interval from 60 s to 1 s — or toggling pause — would see no observable effect until the current sleep finished. Up to a full minute of UI lag.

**Solution:** The background thread sleeps in 250 ms `TICK` chunks instead. Between chunks it re-reads the `refresh_interval` and `paused` atomics; if either changed, the inner sleep breaks and the outer loop re-evaluates state from the top. The TopBar's pause toggle invokes `set_paused`, which emits a `paused-changed` event so other windows (notably the tray popup, which has its own Svelte runtime) update their local state too. Worst-case latency for either signal: ~250 ms; idle CPU cost: one syscall per quarter-second.

## Engineering Decisions

### Tauri v2 over Electron or pure-Rust GUI
- **Constraint**: Wanted a polished, modern UI with Chart.js-quality visualizations, but the system-query layer had to be native (sysinfo) and the binary had to stay small.
- **Options**: Electron + Node bindings to sysinfo, eframe/egui pure-Rust GUI, or Tauri v2 + SvelteKit.
- **Choice**: Tauri v2 with a SvelteKit frontend.
- **Why**: Electron ships its own Chromium (~100MB+ baseline, higher RAM). eframe lacks the design and charting ecosystem I wanted. Tauri reuses the OS's native webview, keeps the Rust core, and ships around ~50MB with the same UI flexibility as Electron.

### Workspace with a reusable library crate
- **Constraint**: I wanted the system-monitoring logic usable outside the desktop app (CLI tools, scripts, examples).
- **Options**: Single binary crate that hides everything in `src-tauri/`, or a Cargo workspace exposing a library plus a thin Tauri binary.
- **Choice**: Workspace — `rust_dashboard_lib` (library) plus `rust-dashboard` (Tauri binary in `src-tauri/`).
- **Why**: The library compiles and is testable without pulling in Tauri. `examples/basic_usage.rs` demonstrates headless use, and integration tests target the library directly via `cargo test -p rust_dashboard_lib`.

### Single broadcast event over per-window polling
- **Constraint**: Main window, five detachable panels, and the tray popup all need to display the same data, in sync, without each spawning its own refresh loop.
- **Options**: Per-window `setInterval` polling, per-window IPC subscriptions, or one background thread broadcasting events.
- **Choice**: A single Rust background thread that builds a `SystemSnapshot` and broadcasts a `system-update` event to all windows via `app_handle.emit()`.
- **Why**: One mutex acquisition per tick, one snapshot, one emit. Detached windows opened later subscribe to the same event and immediately stay in sync — no per-window state-synchronization code.

### Close/recreate tray popup instead of show/hide
- **Constraint**: On multi-monitor setups with mixed DPI (e.g., Retina laptop + external 1x display), a hidden popup retained its old window position and scale factor when shown again.
- **Options**: Reposition on show, listen for monitor-change events, or destroy and recreate the popup each click.
- **Choice**: Destroy on dismiss, recreate from scratch on each tray click, recomputing the target monitor and scale factor every time.
- **Why**: Show/hide preserves stale positioning state that's surprisingly hard to reset cleanly across Tauri/macOS interactions. Recreating is cheap (small SvelteKit bundle) and guarantees correct placement.

### Aggregate-by-name kill semantics
- **Constraint**: The process table aggregates by name to match how users think about apps (one "Chrome" row that summarizes 30 helper PIDs, not 30 separate rows). That aggregation made the kill flow ambiguous — clicking Terminate on the row implied terminating the app, but the underlying command takes a single PID.
- **Options**: Render one row per PID (matching macOS Activity Monitor), keep aggregation and only kill `pids[0]`, or keep aggregation and kill every PID under the row.
- **Choice**: Aggregate AND kill them all. The confirm dialog shows the instance count, the dispatch iterates sequentially, and the event payload reports per-PID success/failure.
- **Why**: Aggregation makes the table scannable for users with many helper-PID apps open. But the prior behavior — killing only `pids[0]` — was confusing in practice: users clicked Terminate, the row stayed, they clicked again, and so on. Committing to "the row IS the process" keeps the table internally consistent.

## Frequently Asked Questions

### Q: Why Tauri v2 instead of Electron?

Tauri uses each platform's native webview instead of bundling Chromium. The result is a ~50MB binary (vs 100MB+ for Electron) that uses significantly less memory. The Rust backend provides direct system access through sysinfo without spawning child processes.

### Q: Why SvelteKit instead of React/Vue?

Svelte's reactive stores map perfectly to real-time data streams — when a `system-update` event arrives, the store updates and all subscribed components re-render automatically. No virtual DOM diffing overhead. SvelteKit's adapter-static generates the static files Tauri needs.

### Q: How does the detachable panel system work?

Each panel has a detach button that creates a new `WebviewWindow` with URL parameters (`?view=cpu&detached=true`). The SvelteKit route reads these parameters and renders only the requested panel. The new window receives the same `system-update` broadcast events as the main window, so it stays in sync automatically.

### Q: How accurate are the CPU percentages?

sysinfo reports CPU usage as the percentage of one core. On a 4-core system, a process using 2 full cores shows 200%. I display it as-is because developers typically want to know "how many cores is this using?"

### Q: How does the process kill safety work?

Four layers: (1) the public library method `SystemMonitor::kill_process` refuses PID 0 and 1, (2) the Tauri command wrapper re-checks as defense in depth, (3) the UI requires explicit user approval through `KillConfirmDialog`, (4) split Tauri capabilities mean detached panels don't have kill permissions at all — only the main window does.

### Q: What happens when I kill a process that has multiple instances?

The process table aggregates by name (like Activity Monitor), so a row labeled "Chrome" can correspond to 30+ helper PIDs. The kill flow honors the aggregation: the confirm dialog shows the instance count ("Terminate All Instances?"), and `KillConfirmDialog.handleKill` iterates `kill_process` sequentially over every PID. Per-PID failures (e.g. permission denied on a privileged child) are logged without aborting the rest of the batch; the dispatched `killed` event carries both the success count and the failure count.

### Q: Can I use SystemMonitor as a library?

Yes. The `rust_dashboard_lib` crate exposes `SystemMonitor`, `AppConfig`, and `DashboardError` independently of Tauri. See `examples/basic_usage.rs` for usage without the GUI.

### Q: What's the test coverage?

43 tests in the library crate: 21 in `tests/test_system.rs` (system metrics, mutex/concurrency, invariants), 5 in `tests/test_config.rs` (TOML round-trip with `tempfile` isolation), 4 in `tests/test_process_actions.rs` (kill safety, lookup edge cases), 3 in `tests/test_export.rs` (serialization formats), plus 10 doctests embedded in `src/system.rs`. Run with `cargo test -p rust_dashboard_lib`.

### Q: How do I build for production?

`cargo tauri build` produces a native `.app` (macOS), `.msi` (Windows), or `.deb`/`.AppImage` (Linux). The release profile uses LTO, strip, and panic=abort for optimized binaries.
