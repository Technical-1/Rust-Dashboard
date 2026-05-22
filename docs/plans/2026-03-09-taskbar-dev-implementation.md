# Taskbar Dev — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a standalone macOS menu bar app that discovers running dev servers and previews them in an embedded tray popup webview.

**Architecture:** Tauri v2 tray-only app (no main window) with SvelteKit frontend. Rust backend scans processes and ports for dev servers every 5 seconds, emits results to frontend. Frontend shows server list → click transitions to iframe preview of the selected server. Config persisted as JSON file.

**Tech Stack:** Tauri v2, SvelteKit 2 + Svelte 5, adapter-static, sysinfo crate, serde_json, dirs crate

**New repo location:** `/Users/jacobkanfer/Desktop/CodeRepositories/Taskbar-Dev`

---

### Task 1: Scaffold the Tauri + SvelteKit project

**Files:**
- Create: `Taskbar-Dev/` (entire project scaffold)

**Step 1: Initialize the project with Tauri CLI**

```bash
cd /Users/jacobkanfer/Desktop/CodeRepositories
mkdir Taskbar-Dev && cd Taskbar-Dev
git init
```

**Step 2: Create the SvelteKit frontend**

```bash
cd /Users/jacobkanfer/Desktop/CodeRepositories/Taskbar-Dev
npm create svelte@latest ui -- --template skeleton --types typescript
cd ui
npm install
npm install @tauri-apps/api@^2.0.0
npm install -D @sveltejs/adapter-static@^3.0.0 @sveltejs/vite-plugin-svelte@^5.0.0
```

**Step 3: Configure SvelteKit for Tauri**

Create `ui/svelte.config.js`:
```javascript
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({ fallback: 'index.html' })
	}
};
export default config;
```

Create `ui/src/routes/+layout.ts`:
```typescript
export const prerender = false;
export const ssr = false;
```

Create `ui/vite.config.ts`:
```typescript
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: { port: 5174, strictPort: true },
	envPrefix: ['VITE_', 'TAURI_']
});
```

Note: Using port 5174 to avoid conflict with Rust Dashboard's 5173.

**Step 4: Create the Tauri backend**

```bash
cd /Users/jacobkanfer/Desktop/CodeRepositories/Taskbar-Dev
cargo tauri init
```

When prompted:
- App name: `taskbar-dev`
- Window title: `Taskbar Dev`
- Frontend dev URL: `http://localhost:5174`
- Frontend dist: `../ui/build`
- Dev command: `cd ../ui && npm run dev`
- Build command: `cd ../ui && npm run build`

**Step 5: Configure Cargo.toml for src-tauri**

Edit `src-tauri/Cargo.toml` dependencies:
```toml
[package]
name = "taskbar-dev"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sysinfo = "0.33"
log = "0.4"
env_logger = "0.11"
dirs = "5.0"

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]
```

**Step 6: Remove default main window from tauri.conf.json**

The tray popup is the only window and is created dynamically. Set `"windows": []` in tauri.conf.json. Also update the CSP to allow loading external localhost URLs in iframes:

```json
{
  "productName": "Taskbar Dev",
  "identifier": "com.taskbardev.app",
  "build": {
    "frontendDist": "../ui/build",
    "devUrl": "http://localhost:5174",
    "beforeDevCommand": "cd ../ui && npm run dev",
    "beforeBuildCommand": "cd ../ui && npm run build"
  },
  "app": {
    "windows": [],
    "security": {
      "csp": "default-src 'self' tauri: asset:; style-src 'self' 'unsafe-inline'; script-src 'self'; img-src 'self' data: blob: asset:; connect-src ipc: http://ipc.localhost; frame-src http://localhost:* http://127.0.0.1:*"
    }
  }
}
```

**Step 7: Add a tray icon**

Copy or create a 32x32 RGBA PNG icon at `src-tauri/icons/icon.png`. You can use the Rust Dashboard icon initially or create a simple one.

**Step 8: Build the frontend once to create ui/build/**

```bash
cd /Users/jacobkanfer/Desktop/CodeRepositories/Taskbar-Dev/ui
npm run build
```

**Step 9: Verify it compiles**

```bash
cd /Users/jacobkanfer/Desktop/CodeRepositories/Taskbar-Dev/src-tauri
cargo check
```

**Step 10: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri + SvelteKit project"
```

---

### Task 2: Tray icon with popup window (no content yet)

**Files:**
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/capabilities/default.json`

**Step 1: Write main.rs with tray icon and popup toggle**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const POPUP_WIDTH: f64 = 400.0;
const POPUP_HEIGHT: f64 = 600.0;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let app = tauri::Builder::default()
        .setup(move |app| {
            let handle = app.handle().clone();

            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or("No default window icon found")?;

            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;

            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("Taskbar Dev")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app_handle, event| {
                    if event.id.as_ref() == "quit" {
                        app_handle.exit(0);
                    }
                })
                .on_tray_icon_event(move |_tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        let tray_handle = handle.clone();

                        // Physical coords from tray rect
                        let (phys_x, phys_y) = match rect.position {
                            tauri::Position::Physical(p) => (p.x as f64, p.y as f64),
                            tauri::Position::Logical(p) => (p.x, p.y),
                        };
                        let (phys_w, phys_h) = match rect.size {
                            tauri::Size::Physical(s) => (s.width as f64, s.height as f64),
                            tauri::Size::Logical(s) => (s.width, s.height),
                        };

                        // Scale factor
                        let scale = tray_handle
                            .available_monitors()
                            .unwrap_or_default()
                            .iter()
                            .find(|m| {
                                let pos = m.position();
                                let sz = m.size();
                                let ix = phys_x as i32;
                                let iy = phys_y as i32;
                                ix >= pos.x
                                    && ix < pos.x + sz.width as i32
                                    && iy >= pos.y
                                    && iy < pos.y + sz.height as i32
                            })
                            .map(|m| m.scale_factor())
                            .unwrap_or(2.0);

                        let icon_x = phys_x / scale;
                        let icon_y = phys_y / scale;
                        let icon_w = phys_w / scale;
                        let icon_h = phys_h / scale;

                        let x = icon_x + (icon_w / 2.0) - (POPUP_WIDTH / 2.0);
                        let y = icon_y + icon_h;

                        // Toggle existing popup
                        if let Some(popup) = tray_handle.get_webview_window("tray-popup") {
                            if popup.is_visible().unwrap_or(false) {
                                let _ = popup.close();
                                return;
                            }
                            let _ = popup.close();
                        }

                        if let Ok(popup) = WebviewWindowBuilder::new(
                            &tray_handle,
                            "tray-popup",
                            WebviewUrl::App("/".into()),
                        )
                        .title("Taskbar Dev")
                        .inner_size(POPUP_WIDTH, POPUP_HEIGHT)
                        .position(x, y)
                        .decorations(false)
                        .shadow(false)
                        .resizable(true)
                        .always_on_top(true)
                        .visible(true)
                        .build()
                        {
                            let _ = popup.set_size(tauri::LogicalSize::new(POPUP_WIDTH, POPUP_HEIGHT));
                            let _ = popup.set_position(tauri::LogicalPosition::new(x, y));
                            let _ = popup.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error building Tauri application");

    // Prevent exit when no windows are open (tray-only app)
    app.run(|_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
```

**Step 2: Create capabilities**

Create `src-tauri/capabilities/default.json`:
```json
{
  "identifier": "default",
  "description": "Default capabilities for tray popup",
  "windows": ["tray-popup"],
  "permissions": [
    "core:default",
    "core:window:default",
    "core:window:allow-close",
    "core:window:allow-set-size",
    "core:window:allow-set-position",
    "core:window:allow-set-focus",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:event:default"
  ]
}
```

**Step 3: Create minimal frontend placeholder**

Create `ui/src/routes/+page.svelte`:
```svelte
<div class="container">
  <h1>Taskbar Dev</h1>
  <p>Scanning for dev servers...</p>
</div>

<style>
  .container {
    padding: 16px;
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
    color: white;
    background: #1a1a2e;
    height: 100vh;
  }
</style>
```

**Step 4: Verify it runs**

```bash
cd /Users/jacobkanfer/Desktop/CodeRepositories/Taskbar-Dev
cargo tauri dev
```

Click the tray icon — popup should appear with placeholder text.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: tray icon with resizable popup window"
```

---

### Task 3: Dev server discovery — Rust backend

**Files:**
- Create: `src-tauri/src/discovery.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Create discovery.rs with data types and process scanning**

```rust
use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerStatus {
    Online,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerSource {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevServer {
    pub id: String,
    pub name: String,
    pub port: u16,
    pub host: String,
    pub project_dir: Option<String>,
    pub status: ServerStatus,
    pub source: ServerSource,
}

/// Known dev server signatures: (process name pattern, framework name, default port)
const KNOWN_SERVERS: &[(&str, &str, u16)] = &[
    ("vite", "Vite", 5173),
    ("next", "Next.js", 3000),
    ("nuxt", "Nuxt", 3000),
    ("webpack", "Webpack", 8080),
    ("react-scripts", "CRA", 3000),
    ("angular", "Angular", 4200),
    ("svelte", "SvelteKit", 5173),
    ("astro", "Astro", 4321),
    ("remix", "Remix", 3000),
    ("gatsby", "Gatsby", 8000),
    ("flask", "Flask", 5000),
    ("django", "Django", 8000),
    ("rails", "Rails", 3000),
    ("cargo-tauri", "Tauri", 1420),
    ("php artisan serve", "Laravel", 8000),
    ("live-server", "Live Server", 8080),
];

/// Common dev ports to scan when process inspection doesn't find them
const SCAN_PORTS: &[u16] = &[
    3000, 3001, 3002, 3003, 3004, 3005,
    4200, 4321,
    5000, 5173, 5174,
    8000, 8080, 8081, 8888,
];

pub struct Scanner {
    sys: System,
}

impl Scanner {
    pub fn new() -> Self {
        Self {
            sys: System::new_with_specifics(
                RefreshKind::nothing().with_processes(
                    ProcessRefreshKind::nothing()
                        .with_cmd(UpdateKind::Always)
                        .with_cwd(UpdateKind::Always),
                ),
            ),
        }
    }

    /// Refresh process list and discover dev servers
    pub fn discover(&mut self) -> Vec<DevServer> {
        self.sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cmd(UpdateKind::Always)
                .with_cwd(UpdateKind::Always),
        );

        let mut servers = Vec::new();
        let mut found_ports = std::collections::HashSet::new();

        // Phase 1: Process inspection
        for (_, process) in self.sys.processes() {
            let cmd_str = process.cmd().join(" ").to_lowercase();

            for &(pattern, name, default_port) in KNOWN_SERVERS {
                if cmd_str.contains(pattern) {
                    let port = extract_port_from_cmd(&cmd_str).unwrap_or(default_port);

                    if found_ports.contains(&port) {
                        continue;
                    }

                    let project_dir = process
                        .cwd()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string());

                    let status = if check_port("127.0.0.1", port) {
                        ServerStatus::Online
                    } else {
                        ServerStatus::Offline
                    };

                    found_ports.insert(port);
                    servers.push(DevServer {
                        id: format!("auto-{}-{}", name.to_lowercase().replace('.', ""), port),
                        name: name.to_string(),
                        port,
                        host: "localhost".to_string(),
                        project_dir,
                        status,
                        source: ServerSource::Auto,
                    });
                    break;
                }
            }
        }

        // Phase 2: Port scanning for ports not already found
        for &port in SCAN_PORTS {
            if found_ports.contains(&port) {
                continue;
            }
            if check_port("127.0.0.1", port) {
                found_ports.insert(port);
                servers.push(DevServer {
                    id: format!("scan-{}", port),
                    name: format!("Port {}", port),
                    port,
                    host: "localhost".to_string(),
                    project_dir: None,
                    status: ServerStatus::Online,
                    source: ServerSource::Auto,
                });
            }
        }

        servers
    }
}

/// Try to extract --port NNNN or -p NNNN from command line
fn extract_port_from_cmd(cmd: &str) -> Option<u16> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if (*part == "--port" || *part == "-p") && i + 1 < parts.len() {
            return parts[i + 1].parse().ok();
        }
        if let Some(rest) = part.strip_prefix("--port=") {
            return rest.parse().ok();
        }
    }
    None
}

/// Check if a TCP port is open (fast timeout)
fn check_port(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    TcpStream::connect_timeout(
        &addr.parse().unwrap(),
        Duration::from_millis(100),
    )
    .is_ok()
}
```

**Step 2: Add discovery module and Tauri commands to main.rs**

Add to main.rs:

```rust
mod discovery;

use discovery::{DevServer, Scanner, ServerSource, ServerStatus};
use std::sync::{Arc, Mutex};

struct AppState {
    scanner: Arc<Mutex<Scanner>>,
    manual_servers: Arc<Mutex<Vec<DevServer>>>,
}
```

Add Tauri commands:

```rust
#[tauri::command]
fn scan_servers(state: tauri::State<'_, AppState>) -> Result<Vec<DevServer>, String> {
    let mut scanner = state.scanner.lock().map_err(|e| e.to_string())?;
    let mut servers = scanner.discover();

    // Append manual servers with fresh status checks
    let manual = state.manual_servers.lock().map_err(|e| e.to_string())?;
    for ms in manual.iter() {
        let status = if check_port_public(&ms.host, ms.port) {
            ServerStatus::Online
        } else {
            ServerStatus::Offline
        };
        servers.push(DevServer {
            status,
            ..ms.clone()
        });
    }

    Ok(servers)
}

#[tauri::command]
fn add_manual_server(
    state: tauri::State<'_, AppState>,
    name: String,
    host: String,
    port: u16,
) -> Result<DevServer, String> {
    let server = DevServer {
        id: format!("manual-{}-{}", host, port),
        name,
        port,
        host: host.clone(),
        project_dir: None,
        status: if check_port_public(&host, port) {
            ServerStatus::Online
        } else {
            ServerStatus::Offline
        },
        source: ServerSource::Manual,
    };

    let mut manual = state.manual_servers.lock().map_err(|e| e.to_string())?;
    // Don't add duplicates
    if !manual.iter().any(|s| s.host == server.host && s.port == server.port) {
        manual.push(server.clone());
    }
    Ok(server)
}

#[tauri::command]
fn remove_manual_server(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mut manual = state.manual_servers.lock().map_err(|e| e.to_string())?;
    manual.retain(|s| s.id != id);
    Ok(())
}

fn check_port_public(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    if let Ok(addr) = addr.parse() {
        std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(100)).is_ok()
    } else {
        false
    }
}
```

Register commands and state in `main()`:

```rust
let app_state = AppState {
    scanner: Arc::new(Mutex::new(Scanner::new())),
    manual_servers: Arc::new(Mutex::new(Vec::new())),
};

// In tauri::Builder:
.manage(app_state)
.invoke_handler(tauri::generate_handler![
    scan_servers,
    add_manual_server,
    remove_manual_server,
])
```

**Step 3: Verify it compiles**

```bash
cd /Users/jacobkanfer/Desktop/CodeRepositories/Taskbar-Dev/src-tauri
cargo check
```

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: dev server discovery via process inspection and port scanning"
```

---

### Task 4: Config persistence (manual servers + window size)

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/main.rs`

**Step 1: Create config.rs**

```rust
use crate::discovery::DevServer;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub manual_servers: Vec<DevServer>,
    pub popup_width: f64,
    pub popup_height: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            manual_servers: Vec::new(),
            popup_width: 400.0,
            popup_height: 600.0,
        }
    }
}

impl AppConfig {
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("taskbar-dev").join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Cannot determine config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }
}
```

**Step 2: Load config on startup, save manual servers on change**

In `main.rs`, load config at startup and initialize `manual_servers` from it. Add a `save_config` command:

```rust
#[tauri::command]
fn save_config(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let manual = state.manual_servers.lock().map_err(|e| e.to_string())?;
    let config = AppConfig {
        manual_servers: manual.clone(),
        popup_width: POPUP_WIDTH,
        popup_height: POPUP_HEIGHT,
    };
    config.save()
}
```

And on startup:
```rust
let config = AppConfig::load();
let app_state = AppState {
    scanner: Arc::new(Mutex::new(Scanner::new())),
    manual_servers: Arc::new(Mutex::new(config.manual_servers)),
};
```

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: persist manual servers and popup size in config"
```

---

### Task 5: Frontend — Server list view

**Files:**
- Create: `ui/src/lib/types.ts`
- Create: `ui/src/lib/components/ServerList.svelte`
- Modify: `ui/src/routes/+page.svelte`
- Create: `ui/src/app.css`

**Step 1: Create types.ts**

```typescript
export interface DevServer {
    id: string;
    name: string;
    port: number;
    host: string;
    project_dir: string | null;
    status: 'Online' | 'Offline';
    source: 'Auto' | 'Manual';
}
```

**Step 2: Create app.css with dark theme**

Create `ui/src/app.css` with a dark theme matching macOS menu bar aesthetic. Include design tokens from Rust Dashboard's pattern (--space-*, --text-*, --radius-*, etc.) but with a darker palette suited for a tray popup.

**Step 3: Create ServerList.svelte**

This component:
- Calls `scan_servers` on mount and every 5 seconds
- Renders each server as a clickable row with: status dot (green/red), name, `:port`, project dir name
- Dispatches `select` event when a server is clicked
- Has an "Add Server" button at the bottom

```svelte
<script lang="ts">
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import type { DevServer } from '$lib/types';

    const dispatch = createEventDispatcher<{ select: DevServer }>();

    let servers: DevServer[] = [];
    let timer: ReturnType<typeof setInterval> | null = null;

    async function refresh() {
        try {
            servers = await invoke<DevServer[]>('scan_servers');
        } catch (e) {
            console.error('scan failed', e);
        }
    }

    onMount(() => {
        refresh();
        timer = setInterval(refresh, 5000);
    });

    onDestroy(() => {
        if (timer) clearInterval(timer);
    });
</script>

<div class="server-list">
    <div class="header">
        <span class="title">Dev Servers</span>
        <span class="count">{servers.length}</span>
    </div>

    {#if servers.length === 0}
        <div class="empty">No dev servers found</div>
    {/if}

    {#each servers as server}
        <button
            class="server-row"
            on:click={() => dispatch('select', server)}
            disabled={server.status === 'Offline'}
        >
            <span class="status-dot" class:online={server.status === 'Online'}></span>
            <div class="server-info">
                <span class="server-name">{server.name}</span>
                <span class="server-port">:{server.port}</span>
            </div>
            {#if server.project_dir}
                <span class="project-dir">{server.project_dir}</span>
            {/if}
        </button>
    {/each}
</div>
```

Add appropriate styles for the dark menu bar aesthetic.

**Step 4: Wire up +page.svelte**

```svelte
<script lang="ts">
    import ServerList from '$lib/components/ServerList.svelte';
    import type { DevServer } from '$lib/types';

    let selectedServer: DevServer | null = null;

    function handleSelect(event: CustomEvent<DevServer>) {
        selectedServer = event.detail;
    }

    function handleBack() {
        selectedServer = null;
    }
</script>

{#if selectedServer}
    <div class="preview-container">
        <div class="preview-header">
            <button class="back-btn" on:click={handleBack}>
                ← Back
            </button>
            <span class="preview-title">{selectedServer.name} :{selectedServer.port}</span>
        </div>
        <iframe
            src="http://{selectedServer.host}:{selectedServer.port}"
            title="{selectedServer.name} preview"
            class="preview-frame"
        ></iframe>
    </div>
{:else}
    <ServerList on:select={handleSelect} />
{/if}
```

**Step 5: Verify it runs**

```bash
cd /Users/jacobkanfer/Desktop/CodeRepositories/Taskbar-Dev
cargo tauri dev
```

Start a dev server on any common port and verify it appears in the list.

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: server list UI with auto-discovery and embedded preview"
```

---

### Task 6: Frontend — Add Server modal

**Files:**
- Create: `ui/src/lib/components/AddServerModal.svelte`
- Modify: `ui/src/lib/components/ServerList.svelte`

**Step 1: Create AddServerModal.svelte**

Simple modal/overlay with fields for: Name, Host (default "localhost"), Port. Calls `add_manual_server` command on submit.

**Step 2: Add "+" button to ServerList that opens the modal**

**Step 3: Verify manually adding a server works**

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: add server modal for manual server entries"
```

---

### Task 7: Frontend — Polish and slide transitions

**Files:**
- Modify: `ui/src/routes/+page.svelte`
- Modify: `ui/src/lib/components/ServerList.svelte`
- Modify: `ui/src/app.css`

**Step 1: Add slide transition between list and preview**

Use Svelte `fly` transition so the list slides left when selecting a server, and the preview slides in from the right. The back button slides the preview out and brings the list back.

**Step 2: Style the preview header to match the tray aesthetic**

Small, dark header bar with back arrow, server name, and port. The iframe should fill the remaining space.

**Step 3: Add right-click context menu for manual servers (remove option)**

On right-click of a manual server row, show a context option to remove it.

**Step 4: Polish the empty state**

When no servers are found, show a friendly message with a suggestion to start a dev server or add one manually.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: slide transitions and UI polish"
```

---

### Task 8: Window size persistence

**Files:**
- Modify: `src-tauri/src/main.rs`

**Step 1: Save window size on resize**

Listen for resize events on the tray popup window. When the user resizes, save the new dimensions to config. On next popup creation, use the saved dimensions instead of defaults.

Add to main.rs setup, after popup is created:
```rust
// Listen for resize to persist popup size
let resize_handle = app.handle().clone();
app.listen("popup-resized", move |event| {
    // Save to config
});
```

Alternatively, read the popup size on close and persist it.

**Step 2: Load saved size on popup creation**

Replace the hardcoded `POPUP_WIDTH`/`POPUP_HEIGHT` with values loaded from config.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: persist popup window size across sessions"
```

---

### Task 9: Close popup on focus loss

**Files:**
- Modify: `src-tauri/src/main.rs`

**Step 1: Auto-close popup when it loses focus**

This is critical for tray popup UX — clicking anywhere outside should dismiss it, just like native macOS menu bar popups.

After creating the popup window, listen for the `FocusChanged` window event:

```rust
popup.on_window_event(move |event| {
    if let WindowEvent::Focused(false) = event {
        // Close popup when it loses focus
        if let Some(win) = close_handle.get_webview_window("tray-popup") {
            let _ = win.close();
        }
    }
});
```

**Step 2: Test focus-loss behavior**

Click tray icon → popup appears. Click elsewhere → popup should close.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat: close popup on focus loss"
```

---

### Task 10: Final integration testing and .gitignore

**Files:**
- Create: `.gitignore`
- Modify: various

**Step 1: Create .gitignore**

```
/target
/ui/node_modules
/ui/build
/ui/.svelte-kit
.DS_Store
*.log
```

**Step 2: Full integration test**

1. Run `cargo tauri dev`
2. Start a Vite dev server on port 5173 in another project
3. Verify it appears in the server list within 5 seconds
4. Click it → verify iframe preview loads
5. Click back → verify return to list
6. Add a manual server → verify it persists after quit/restart
7. Resize popup → verify size persists after quit/restart
8. Click outside popup → verify it closes

**Step 3: Final commit**

```bash
git add -A
git commit -m "chore: gitignore and integration testing"
```

---

## Summary of Tasks

| Task | Description | Est. |
|------|-------------|------|
| 1 | Scaffold Tauri + SvelteKit project | 10 min |
| 2 | Tray icon with popup window | 10 min |
| 3 | Dev server discovery (Rust) | 15 min |
| 4 | Config persistence | 5 min |
| 5 | Server list + embedded preview UI | 15 min |
| 6 | Add Server modal | 10 min |
| 7 | Slide transitions + polish | 10 min |
| 8 | Window size persistence | 5 min |
| 9 | Close popup on focus loss | 5 min |
| 10 | Integration testing + gitignore | 5 min |
