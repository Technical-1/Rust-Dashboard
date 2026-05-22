# Taskbar Dev — Design Document

## Overview
A standalone macOS menu bar app that discovers running dev servers and lets you preview them directly inside the tray popup. Compact server list view transitions to an embedded webview preview on selection.

## Architecture

- **Standalone Tauri v2 app** in a separate repository (`taskbar-dev`)
- **Rust backend**: dev server discovery (process inspection + port scanning), manual server management, persistent config
- **SvelteKit frontend**: two views within the tray popup — server list and embedded preview

## Dev Server Discovery

### Auto-discovery (every 5 seconds)
1. Scan running processes for known dev server signatures (`node.*vite`, `node.*next`, `node.*webpack`, `cargo.*tauri`, `python.*manage.py runserver`, `ruby.*rails s`, etc.)
2. Extract port from process args or known defaults
3. Port scan configurable list of common ports (3000-3010, 4200, 5173-5174, 8000, 8080, etc.) with HTTP health check
4. Deduplicate — process-detected servers take priority (richer metadata)

### Manual add
- User can add arbitrary `host:port` or full URL
- Persisted in config file

### Server data model
```rust
struct DevServer {
    name: String,           // framework name or "Custom"
    port: u16,
    host: String,           // default "localhost"
    project_dir: Option<String>,  // from process cwd
    status: ServerStatus,   // Online | Offline
    source: ServerSource,   // Auto | Manual
}
```

## UI Flow

1. **Click tray icon** → popup appears with server list
2. **Server list**: status dot (green/red) + name + port + project directory name
3. **Click a server** → list slides out, webview loads server URL, back button appears
4. **Back button** → returns to server list
5. **Right-click tray icon** → context menu: "Add Server...", "Settings", "Quit"
6. **Popup is resizable**, remembers last size (default 400x600)

## Tech Stack
- Tauri v2 (tray-icon, image-png features)
- SvelteKit + adapter-static
- `sysinfo` crate for process scanning
- `std::net::TcpStream` for port probing
- JSON config file for persistence (manual servers, window size)

## Key Decisions
- Separate repo (no coupling to Rust Dashboard)
- Tray popup embeds webview directly (not separate windows)
- Process inspection + port scanning + manual add for discovery
- Server list with status dot, name, port, project dir
- Resizable popup with size memory
