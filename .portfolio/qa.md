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

Three layers: (1) PID guard rejects PID 0 and 1 in the Tauri command, (2) confirmation dialog in the UI requires explicit user approval, (3) split Tauri capabilities mean detached panels don't have kill permissions at all.

### Q: Can I use SystemMonitor as a library?

Yes. The `rust_dashboard_lib` crate exposes `SystemMonitor`, `AppConfig`, and `DashboardError` independently of Tauri. See `examples/basic_usage.rs` for usage without the GUI.

### Q: What's the test coverage?

40 tests covering system monitoring (unit + doc-tests), configuration persistence, export functionality (JSON + CSV), and process actions. Tests only cover the library crate: `cargo test -p rust_dashboard_lib`.

### Q: How do I build for production?

`cargo tauri build` produces a native `.app` (macOS), `.msi` (Windows), or `.deb`/`.AppImage` (Linux). The release profile uses LTO, strip, and panic=abort for optimized binaries.
