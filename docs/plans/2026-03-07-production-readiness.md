# Production Readiness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix all 19 verified issues from the production readiness audit across security, stability, and quality.

**Architecture:** Three-phase approach. Phase 1 hardens security (capabilities, path validation, PID guards, CSP, CI). Phase 2 fixes stability (exit handling, mutex recovery, error propagation). Phase 3 polishes quality (accessibility, CSV safety, config robustness, CI cleanup, logging). Each task is self-contained with a commit checkpoint.

**Tech Stack:** Rust + Tauri v2 backend, SvelteKit + TypeScript frontend, GitHub Actions CI/CD

**Test command (library):** `cargo test -p rust_dashboard_lib --verbose`
**Build check (binary):** `cargo check -p rust-dashboard`
**Frontend check:** `cd ui && npm run build`

---

## Phase 1: Security (Tasks 1-7)

---

### Task 1: Fix Path Traversal in `export_to_file` [H1]

**Files:**
- Modify: `src-tauri/src/main.rs:215-231`

**Step 1: Replace the blocklist with canonical path allowlisting**

Replace the entire `export_to_file` function (lines 215-231) with:

```rust
#[tauri::command]
fn export_to_file(data: String, path: String) -> Result<(), String> {
    let path_ref = std::path::Path::new(&path);

    // Validate file extension
    match path_ref.extension().and_then(|e| e.to_str()) {
        Some("json") | Some("csv") => {}
        _ => return Err("Only .json and .csv file extensions are allowed".to_string()),
    }

    // Canonicalize the parent directory (file may not exist yet)
    let parent = path_ref
        .parent()
        .ok_or("Invalid path: no parent directory")?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path: {}", e))?;

    // Allow only writes under the user's home directory
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    if !canonical_parent.starts_with(&home) {
        return Err("Writes are only allowed within your home directory".to_string());
    }

    let file_name = path_ref
        .file_name()
        .ok_or("Invalid filename")?;
    let safe_path = canonical_parent.join(file_name);

    std::fs::write(&safe_path, data).map_err(|e| e.to_string())
}
```

**Step 2: Add `dirs` dependency to `src-tauri/Cargo.toml`**

`dirs` is already a dependency of the root workspace crate, but the binary crate `src-tauri` doesn't list it. Check if the `dirs` import resolves through `rust_dashboard_lib`. If not, add to `src-tauri/Cargo.toml` dependencies:

```toml
dirs = "5.0"
```

**Step 3: Verify it compiles**

Run: `cargo check -p rust-dashboard`
Expected: Compiles without errors.

**Step 4: Commit**

```bash
git add src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "fix(security): replace path blocklist with canonical allowlist in export_to_file

Canonicalize the parent directory and only allow writes under \$HOME.
Prevents ../ traversal, symlink attacks, and works cross-platform.
Resolves H1 from production readiness audit."
```

---

### Task 2: Remove Over-Permissive `fs:default` Capability [H2]

**Files:**
- Modify: `src-tauri/capabilities/default.json:19`
- Modify: `src-tauri/src/main.rs:258`
- Modify: `src-tauri/Cargo.toml` (remove `tauri-plugin-fs`)

**Step 1: Remove `fs:default` from capabilities**

In `src-tauri/capabilities/default.json`, remove line 19 (`"fs:default",`). The permissions array becomes:

```json
"permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-start-resize-dragging",
    "core:window:allow-set-resizable",
    "core:window:allow-close",
    "core:window:allow-set-focus",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:webview:allow-create-webview-window",
    "core:event:default",
    "dialog:default",
    "dialog:allow-save",
    "os:default",
    "window-state:default"
]
```

**Step 2: Remove the FS plugin initialization**

In `src-tauri/src/main.rs`, remove line 258:
```rust
.plugin(tauri_plugin_fs::init())
```

**Step 3: Remove the FS plugin dependency**

In `src-tauri/Cargo.toml`, remove the line:
```toml
tauri-plugin-fs = "2"
```

**Step 4: Verify it compiles**

Run: `cargo check -p rust-dashboard`
Expected: Compiles without errors. The export flow uses the Rust `export_to_file` command, not the frontend FS plugin.

**Step 5: Verify the frontend still builds**

Run: `cd ui && npm run build`
Expected: Builds successfully. If any import of `@tauri-apps/plugin-fs` exists in frontend code, remove it (there shouldn't be any — exports use `invoke('export_to_file')`).

**Step 6: Commit**

```bash
git add src-tauri/capabilities/default.json src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "fix(security): remove unnecessary fs:default capability and FS plugin

The export flow uses a Rust command, not frontend FS APIs.
Removes over-permissive filesystem access from all windows.
Resolves H2 from production readiness audit."
```

---

### Task 3: Add PID Guard to `kill_process` [H3]

**Files:**
- Modify: `src-tauri/src/main.rs:162-166`

**Step 1: Add PID validation**

Replace the `kill_process` function (lines 162-166) with:

```rust
#[tauri::command]
fn kill_process(state: tauri::State<'_, AppState>, pid: u32) -> Result<(), String> {
    if pid <= 1 {
        return Err("Cannot terminate system processes (PID 0 or 1)".to_string());
    }
    let mut monitor = state.monitor.lock().map_err(|e| e.to_string())?;
    monitor.kill_process(pid)
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p rust-dashboard`
Expected: Compiles without errors.

**Step 3: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "fix(security): add PID guard to kill_process command

Block killing PID 0 (kernel) and PID 1 (init/launchd/systemd).
Resolves H3 from production readiness audit."
```

---

### Task 4: Split Capabilities by Window Type [H4]

**Files:**
- Delete: `src-tauri/capabilities/default.json`
- Create: `src-tauri/capabilities/main.json`
- Create: `src-tauri/capabilities/panels.json`

**Step 1: Create `main.json` for the main window**

```json
{
  "identifier": "main-window",
  "description": "Capabilities for the main dashboard window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-start-resize-dragging",
    "core:window:allow-set-resizable",
    "core:window:allow-close",
    "core:window:allow-set-focus",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:webview:allow-create-webview-window",
    "core:event:default",
    "dialog:default",
    "dialog:allow-save",
    "os:default",
    "window-state:default"
  ]
}
```

**Step 2: Create `panels.json` for secondary windows**

```json
{
  "identifier": "panel-windows",
  "description": "Capabilities for detached panels and tray popup (no webview creation)",
  "windows": ["detached-*", "tray-popup"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-start-resize-dragging",
    "core:window:allow-set-resizable",
    "core:window:allow-close",
    "core:window:allow-set-focus",
    "core:window:allow-show",
    "core:window:allow-hide",
    "core:event:default",
    "os:default",
    "window-state:default"
  ]
}
```

Note: `dialog:default`, `dialog:allow-save`, and `core:webview:allow-create-webview-window` are intentionally absent from panels — only the main window needs file dialogs and webview creation.

**Step 3: Delete the old `default.json`**

```bash
rm src-tauri/capabilities/default.json
```

**Step 4: Verify it compiles**

Run: `cargo check -p rust-dashboard`
Expected: Compiles. Tauri auto-discovers all `.json` files in the `capabilities/` directory.

**Step 5: Commit**

```bash
git add src-tauri/capabilities/
git commit -m "fix(security): split capabilities to restrict webview creation to main window

Secondary windows (detached panels, tray popup) no longer have
create-webview-window or dialog permissions. Only main window can
spawn new webviews and trigger file save dialogs.
Resolves H4 from production readiness audit."
```

---

### Task 5: Fix Tray Quit to Use Proper Tauri Exit [H5]

**Files:**
- Modify: `src-tauri/src/main.rs:285-287` (tray quit handler)
- Modify: `src-tauri/src/main.rs:446-449` (RunEvent::ExitRequested handler)

**Step 1: Replace `std::process::exit(0)` with `app_handle.exit(0)`**

At line 286, change:
```rust
// Before:
"quit" => {
    std::process::exit(0);
}

// After:
"quit" => {
    app_handle.exit(0);
}
```

**Step 2: Update the `RunEvent::ExitRequested` handler to allow explicit exits**

At lines 446-449, change:
```rust
// Before:
app.run(|_app_handle, event| {
    if let tauri::RunEvent::ExitRequested { api, .. } = event {
        api.prevent_exit();
    }
});

// After:
app.run(|_app_handle, event| {
    if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
        // Only prevent exit for window-close events (code == None).
        // Allow explicit app.exit() calls (code == Some(0)) to proceed.
        if code.is_none() {
            api.prevent_exit();
        }
    }
});
```

**Important context:** The current `RunEvent::ExitRequested` handler prevents ALL exits unconditionally, which is why `std::process::exit` was used as a workaround. By checking `code`, we let `app_handle.exit(0)` pass through while still hiding the window on close.

**Step 3: Verify it compiles**

Run: `cargo check -p rust-dashboard`
Expected: Compiles. The `code` field is available on Tauri v2's `ExitRequested` event.

**Step 4: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "fix(stability): use app_handle.exit() instead of process::exit() for tray quit

process::exit bypasses all Tauri cleanup hooks, window-state saves,
and Drop implementations. Now uses the proper Tauri exit path.
Updated ExitRequested handler to allow explicit exit codes through.
Resolves H5 from production readiness audit."
```

---

### Task 6: Add Security Scanning to CI [H6]

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: Add cargo-audit and npm audit steps**

In `.github/workflows/ci.yml`, add the following steps to the `test` job, after the "Install frontend dependencies" step (after line 59) and before "Build frontend":

```yaml
      - name: Audit Rust dependencies
        run: |
          cargo install cargo-audit --locked || true
          cargo audit

      - name: Audit npm dependencies
        run: cd ui && npm audit --audit-level=moderate || true
```

Note: `|| true` on npm audit prevents CI from failing on advisory-only findings (no fix available). Remove `|| true` if you want strict enforcement. For `cargo install`, `|| true` prevents failure if cargo-audit is already installed from cache.

**Step 2: Verify the YAML is valid**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" 2>&1 || echo "Install pyyaml: pip3 install pyyaml"`

Or just visually verify indentation is correct (2-space indentation under `steps:`).

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add cargo audit and npm audit security scanning

Catches known CVEs in Rust and npm dependencies during CI.
Resolves H6 from production readiness audit."
```

---

### Task 7: Tighten CSP with Tauri Protocol Origins [M7]

**Files:**
- Modify: `src-tauri/tauri.conf.json:26`

**Step 1: Replace the CSP**

At line 26, change:
```json
"csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self'"
```

To:
```json
"csp": "default-src 'self' tauri: asset:; style-src 'self' 'unsafe-inline'; script-src 'self'; img-src 'self' data: blob: asset:; connect-src ipc: http://ipc.localhost"
```

**Why each directive:**
- `tauri:` and `asset:` in `default-src`: Tauri's custom protocol for loading app assets
- `data:` and `blob:` in `img-src`: Allows data URIs and blob URLs for dynamically generated images
- `ipc:` and `http://ipc.localhost` in `connect-src`: Tauri v2's IPC channel origins
- `'unsafe-inline'` kept in `style-src`: Required by Svelte's scoped styles (removing it would break all component styles)

**Step 2: Verify it compiles and runs**

Run: `cargo check -p rust-dashboard`
Expected: Compiles. CSP is a runtime concern — full testing requires running the app and verifying no CSP violations appear in the webview console.

**Step 3: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "fix(security): tighten CSP with explicit Tauri protocol origins

Add tauri:/asset: to default-src, data:/blob: to img-src,
and ipc: to connect-src. Prevents unexpected resource blocking.
Resolves M7 from production readiness audit."
```

---

## Phase 2: Stability (Tasks 8-11)

---

### Task 8: Recover from Mutex Poisoning in Background Thread [M1]

**Files:**
- Modify: `src-tauri/src/main.rs:387-397`

**Step 1: Replace the poison error handler with recovery**

Replace lines 387-397 (the `match monitor.lock()` block) with:

```rust
                        let snapshot = {
                            let mut mon = monitor.lock().unwrap_or_else(|e| {
                                log::warn!("Monitor mutex was poisoned, recovering: {}", e);
                                e.into_inner()
                            });
                            mon.refresh();
                            build_snapshot(&mon)
                        };
```

**Step 2: Also apply to history mutex locks**

Replace lines 404-416 (the two `if let Ok(mut hist)` blocks) with:

```rust
                        // Update history
                        let elapsed = history_start.elapsed().as_secs_f64();
                        {
                            let mut hist = cpu_history.lock().unwrap_or_else(|e| e.into_inner());
                            hist.push_back((elapsed, snapshot.cpu_usage));
                            while hist.len() > HISTORY_CAPACITY {
                                hist.pop_front();
                            }
                        }
                        {
                            let mut hist = memory_history.lock().unwrap_or_else(|e| e.into_inner());
                            let used_gb = snapshot.memory.used as f64 / 1024.0 / 1024.0 / 1024.0;
                            hist.push_back((elapsed, used_gb));
                            while hist.len() > HISTORY_CAPACITY {
                                hist.pop_front();
                            }
                        }
```

**Why `unwrap_or_else(|e| e.into_inner())`:** A poisoned mutex means another thread panicked while holding the lock, but the data inside is still valid and accessible. `into_inner()` recovers the `MutexGuard`, allowing the background thread to continue operating instead of entering a permanent failure loop.

**Step 3: Verify it compiles**

Run: `cargo check -p rust-dashboard`
Expected: Compiles without errors.

**Step 4: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "fix(stability): recover from mutex poisoning in background thread

Use unwrap_or_else(|e| e.into_inner()) instead of returning Err.
Prevents the monitor from entering a permanent failure loop if
any thread panics while holding a lock.
Resolves M1 from production readiness audit."
```

---

### Task 9: Clamp Refresh Interval Upper Bound [M2]

**Files:**
- Modify: `src-tauri/src/main.rs:168-172`

**Step 1: Add upper bound and improve memory ordering**

Replace the `set_refresh_interval` function (lines 168-172) with:

```rust
#[tauri::command]
fn set_refresh_interval(state: tauri::State<'_, AppState>, seconds: u32) {
    let clamped = seconds.clamp(1, 60);
    state.refresh_interval.store(clamped, Ordering::Release);
}
```

**Step 2: Update the background thread's load to use Acquire ordering**

At line 384, change:
```rust
// Before:
let interval_secs = refresh_interval.load(Ordering::Relaxed);

// After:
let interval_secs = refresh_interval.load(Ordering::Acquire);
```

At line 386, change:
```rust
// Before:
if !paused.load(Ordering::Relaxed) {

// After:
if !paused.load(Ordering::Acquire) {
```

And update `set_paused` (lines 174-177) to use Release:
```rust
#[tauri::command]
fn set_paused(state: tauri::State<'_, AppState>, paused: bool) {
    state.paused.store(paused, Ordering::Release);
}
```

**Step 3: Verify it compiles**

Run: `cargo check -p rust-dashboard`
Expected: Compiles without errors.

**Step 4: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "fix(stability): clamp refresh interval to 1-60s, use Release/Acquire ordering

Prevents u32::MAX from freezing the monitor forever.
Upgrades Relaxed atomics to proper Release/Acquire pairs.
Resolves M2 from production readiness audit."
```

---

### Task 10: Wrap `listen()` Calls in try/catch [M3]

**Files:**
- Modify: `ui/src/lib/stores/system.ts:30-51`
- Modify: `ui/src/routes/+page.svelte:57-63`

**Step 1: Wrap listen in system.ts**

In `ui/src/lib/stores/system.ts`, replace lines 30-51 with:

```typescript
	// Listen for push updates
	try {
		unlisten = await listen<SystemSnapshot>('system-update', (event) => {
			const snapshot = event.payload;
			systemSnapshot.set(snapshot);
			systemError.set(null);

			// Append to histories (cap at 300)
			cpuHistory.update((hist) => {
				const now = performance.now() / 1000;
				hist.push([now, snapshot.cpu_usage]);
				if (hist.length > 300) hist.shift();
				return hist;
			});

			memoryHistory.update((hist) => {
				const now = performance.now() / 1000;
				const usedGb = snapshot.memory.used / 1024 / 1024 / 1024;
				hist.push([now, usedGb]);
				if (hist.length > 300) hist.shift();
				return hist;
			});
		});
	} catch (e) {
		console.error('Failed to listen for system updates:', e);
		systemError.set('Failed to connect to system event stream');
	}
```

**Step 2: Wrap listen in +page.svelte**

In `ui/src/routes/+page.svelte`, replace lines 57-63 with:

```typescript
		// Listen for merge-back events from detached windows
		try {
			unlistenMerge = await listen<{ view: string }>('merge-back', (event) => {
				const view = event.payload.view;
				if (['cpu', 'memory', 'disks', 'network', 'processes'].includes(view)) {
					activeView.set(view as DetachableView);
				}
			});
		} catch (e) {
			console.error('Failed to listen for merge-back events:', e);
		}
```

**Step 3: Verify frontend builds**

Run: `cd ui && npm run build`
Expected: Builds without errors.

**Step 4: Commit**

```bash
git add ui/src/lib/stores/system.ts ui/src/routes/+page.svelte
git commit -m "fix(stability): wrap listen() calls in try/catch

Prevents unhandled promise rejections if Tauri IPC event
registration fails. Sets systemError store on failure.
Resolves M3 from production readiness audit."
```

---

### Task 11: Install Panic Hook for Crash Diagnostics [L2]

**Files:**
- Modify: `src-tauri/src/main.rs:235-236`

**Step 1: Add panic hook at the top of main()**

Insert after line 236 (`env_logger::Builder::...init();`):

```rust
    // Install panic hook for crash diagnostics under panic="abort"
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {}", info);
        // Also write to stderr in case the logger isn't working
        eprintln!("PANIC: {}", info);
    }));
```

**Step 2: Verify it compiles**

Run: `cargo check -p rust-dashboard`
Expected: Compiles without errors.

**Step 3: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "fix(stability): install panic hook for crash diagnostics

With panic='abort' in the release profile, panics terminate
immediately. The hook logs diagnostic info before the abort.
Resolves L2 from production readiness audit."
```

---

## Phase 3: Quality (Tasks 12-19)

---

### Task 12: Add Keyboard Support to Sortable Table Headers [M4]

**Files:**
- Modify: `ui/src/lib/components/ProcessTable.svelte:148-180`

**Step 1: Add a keydown handler function**

Add this function after `toggleSort` (after line 60 in the `<script>` block):

```typescript
	function handleSortKeydown(e: KeyboardEvent, col: SortColumn) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			toggleSort(col);
		}
	}
```

**Step 2: Update all four `<th>` elements**

Add `tabindex="0"` and `on:keydown` to each sortable `<th>`. The pattern for each (example for 'name' at line 149):

```svelte
<th class="sortable" class:active-sort={isActiveSort('name')} on:click={() => toggleSort('name')} on:keydown={(e) => handleSortKeydown(e, 'name')} tabindex="0" role="columnheader" aria-sort={isActiveSort('name') ? ($sortDirection === 'asc' ? 'ascending' : 'descending') : 'none'}>
```

Apply the same pattern to all four sortable headers: `'name'` (line 149), `'cpu'` (line 157), `'memory'` (line 165), `'pids'` (line 173).

**Step 3: Verify frontend builds**

Run: `cd ui && npm run build`
Expected: Builds without errors.

**Step 4: Commit**

```bash
git add ui/src/lib/components/ProcessTable.svelte
git commit -m "fix(a11y): add keyboard support to sortable table headers

Add tabindex, keydown handlers (Enter/Space), and role=columnheader
to all sortable th elements. Keyboard users can now sort columns.
Resolves M4 from production readiness audit."
```

---

### Task 13: Fix `config_path()` Silent Fallback [M5]

**Files:**
- Modify: `src/config.rs:28-55`
- Modify: `src/error.rs` (add ConfigError variant)

**Step 1: Add a config error variant**

In `src/error.rs`, add a new variant to `DashboardError`:

```rust
    /// Configuration error (cannot find/create config directory or read/write config file)
    #[error("Configuration error: {0}")]
    ConfigError(String),
```

**Step 2: Refactor `config_path()` to return Result**

In `src/config.rs`, change `config_path` to:

```rust
    pub fn config_path() -> Result<PathBuf, String> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| "Cannot determine config directory".to_string())?;
        path.push("rust-dashboard");
        fs::create_dir_all(&path)
            .map_err(|e| format!("Cannot create config directory: {}", e))?;
        path.push("config.toml");
        Ok(path)
    }
```

**Step 3: Update `load()` to handle the Result**

```rust
    pub fn load() -> Self {
        let path = match Self::config_path() {
            Ok(p) => p,
            Err(e) => {
                log::warn!("Config path error, using defaults: {}", e);
                return Self::default();
            }
        };
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }
```

**Step 4: Update `save()` to handle the Result**

```rust
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }
```

**Step 5: Run tests**

Run: `cargo test -p rust_dashboard_lib --verbose`
Expected: All 40 tests pass. The config tests use `config_path()` internally — verify they still work.

**Step 6: Verify binary compiles**

Run: `cargo check -p rust-dashboard`
Expected: Compiles. `main.rs:238` calls `AppConfig::load()` which still returns `Self` (fallback to defaults).

**Step 7: Commit**

```bash
git add src/config.rs src/error.rs
git commit -m "fix(config): return Result from config_path instead of silent CWD fallback

Propagates config directory errors instead of silently writing
config to the current working directory. Falls back to defaults
with a warning log on error.
Resolves M5 from production readiness audit."
```

---

### Task 14: Fix CSV Formula Injection [M6]

**Files:**
- Modify: `ui/src/lib/components/ExportButtons.svelte:49-55`

**Step 1: Add a CSV escape helper function**

Add this function at the top of the `<script>` block (after line 6):

```typescript
	function csvEscape(val: string): string {
		// Prefix formula-injection characters to prevent execution in spreadsheets
		let safe = val;
		if (/^[=+\-@\t\r]/.test(safe)) {
			safe = "'" + safe;
		}
		// Wrap in double quotes and escape embedded quotes
		return `"${safe.replace(/"/g, '""')}"`;
	}
```

**Step 2: Update the CSV generation loop**

Replace line 54 (the `csv +=` line inside the `for` loop) with:

```typescript
				csv += `${csvEscape('Process')},${csvEscape(p.name)},${p.cpu_usage.toFixed(2)},${Math.floor(p.memory_usage / 1024 / 1024)},${csvEscape(pids)}\n`;
```

Also update lines 50-51 (the System rows) to use the escape function:

```typescript
			csv += `${csvEscape('System')},${csvEscape('CPU')},${$systemSnapshot.cpu_usage.toFixed(2)},,\n`;
			csv += `${csvEscape('System')},${csvEscape('Memory')},,${Math.floor($systemSnapshot.memory.used / 1024 / 1024)},\n`;
```

**Step 3: Verify frontend builds**

Run: `cd ui && npm run build`
Expected: Builds without errors.

**Step 4: Commit**

```bash
git add ui/src/lib/components/ExportButtons.svelte
git commit -m "fix(security): add CSV formula injection escaping to exports

Wrap fields in double-quotes and prefix formula characters (=+\-@)
to prevent spreadsheet formula execution. Uses standard CSV quoting.
Resolves M6 from production readiness audit."
```

---

### Task 15: Delete Redundant `rust.yml` Workflow [L1]

**Files:**
- Delete: `.github/workflows/rust.yml`

**Step 1: Delete the file**

```bash
rm .github/workflows/rust.yml
```

**Step 2: Verify `ci.yml` covers everything `rust.yml` did**

`rust.yml` ran: build (ubuntu only) + tests. `ci.yml` runs: fmt check + tests + build (3-platform matrix) + clippy. It's a strict superset.

**Step 3: Commit**

```bash
git add .github/workflows/rust.yml
git commit -m "ci: remove redundant rust.yml workflow

ci.yml already covers build, test, fmt, and clippy across all
platforms. rust.yml was a subset running on ubuntu-latest only.
Resolves L1 from production readiness audit."
```

---

### Task 16: Pin GitHub Actions to Commit SHAs [L3]

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`

**Step 1: Look up current commit SHAs for each action**

Run:
```bash
# Get the latest v4 SHA for actions/checkout
gh api repos/actions/checkout/git/ref/tags/v4 --jq '.object.sha' 2>/dev/null || echo "Look up manually"
# Get the latest v3 SHA for actions/cache
gh api repos/actions/cache/git/ref/tags/v3 --jq '.object.sha' 2>/dev/null || echo "Look up manually"
```

If `gh` isn't available or tags are annotated, look up the SHAs on GitHub. Replace all `@vN` references with `@<sha> # vN` format.

**Step 2: Update `ci.yml`**

Replace all action version references. Example (use actual SHAs at time of implementation):
```yaml
- uses: actions/checkout@<sha>  # v4
- uses: actions/cache@<sha>  # v3
- uses: dtolnay/rust-toolchain@<sha>  # stable
- uses: actions/setup-node@<sha>  # v4
```

**Step 3: Update `release.yml`**

Same pattern for:
```yaml
- uses: actions/checkout@<sha>  # v4
- uses: dtolnay/rust-toolchain@<sha>  # stable
- uses: actions/setup-node@<sha>  # v4
- uses: tauri-apps/tauri-action@<sha>  # v0
```

**Step 4: Commit**

```bash
git add .github/workflows/ci.yml .github/workflows/release.yml
git commit -m "ci: pin GitHub Actions to immutable commit SHAs

Prevents supply-chain attacks via mutable tag overwriting.
Each action pinned to its current commit SHA with version comment.
Resolves L3 from production readiness audit."
```

---

### Task 17: Replace `console.error` with Conditional Logging [L4]

**Files:**
- Modify: `ui/src/lib/stores/config.ts:17,26,35,51`
- Modify: `ui/src/lib/components/ProcessRow.svelte:24`
- Modify: `ui/src/lib/components/ExportButtons.svelte:38,64`
- Modify: `ui/src/lib/components/TopBar.svelte:16`
- Modify: `ui/src/lib/components/KillConfirmDialog.svelte:18`
- Modify: `ui/src/lib/components/TrayPopup.svelte:20`

**Step 1: Create a logging utility**

Create `ui/src/lib/log.ts`:

```typescript
export function logError(context: string, e: unknown) {
	if (import.meta.env.DEV) {
		console.error(`${context}:`, e);
	}
}
```

**Step 2: Replace all `console.error` calls**

In each file, replace `console.error('message', e)` with `logError('message', e)` and add the import:

```typescript
import { logError } from '$lib/log';
```

Files and their replacements:
- `config.ts:17` — `logError('Failed to load config', e)`
- `config.ts:26` — `logError('Failed to set refresh interval', e)`
- `config.ts:35` — `logError('Failed to toggle pause', e)`
- `config.ts:51` — `logError('Failed to save config', e)`
- `ProcessRow.svelte:24` — `logError('Failed to load process details', e)`
- `ExportButtons.svelte:38` — `logError('Export JSON failed', e)`
- `ExportButtons.svelte:64` — `logError('Export CSV failed', e)`
- `TopBar.svelte:16` — `logError('Manual refresh failed', e)`
- `KillConfirmDialog.svelte:18` — `logError('Failed to kill process', e)`
- `TrayPopup.svelte:20` — `logError('tray_refresh failed', e)`

Also update `system.ts:26` (already in a try/catch) to use: `logError('Failed to fetch initial data', e)`.

Also update the new try/catch blocks added in Task 10 (`system.ts` and `+page.svelte`) to use `logError`.

**Step 3: Verify frontend builds**

Run: `cd ui && npm run build`
Expected: Builds without errors.

**Step 4: Commit**

```bash
git add ui/src/lib/log.ts ui/src/lib/stores/ ui/src/lib/components/ ui/src/routes/
git commit -m "fix(logging): replace console.error with conditional dev-only logging

Create logError utility that only logs in dev mode.
Prevents internal error details from leaking to DevTools in production.
Resolves L4 from production readiness audit."
```

---

### Task 18: Add `aria-label` Prop to ProgressBar [L5]

**Files:**
- Modify: `ui/src/lib/components/ProgressBar.svelte:2,6`
- Modify: `ui/src/lib/components/CpuPanel.svelte` (ProgressBar usage)
- Modify: `ui/src/lib/components/MemoryPanel.svelte` (ProgressBar usage)
- Modify: `ui/src/lib/components/DiskPanel.svelte` (ProgressBar usage)

**Step 1: Add the aria-label prop to ProgressBar**

In `ProgressBar.svelte`, add a new prop at line 3:

```typescript
	export let label: string = 'Progress';
```

Update line 6 to include it:

```svelte
<div class="progress-track" role="progressbar" aria-valuenow={Math.round(value * 100)} aria-valuemin={0} aria-valuemax={100} aria-label={label}>
```

**Step 2: Pass labels from each usage site**

In `CpuPanel.svelte` (line 48):
```svelte
<ProgressBar value={cpu / 100} color={getStatusColor(cpu)} label="CPU usage {cpu.toFixed(0)}%" />
```

In `MemoryPanel.svelte` (line 60):
```svelte
<ProgressBar value={percent / 100} color={getStatusColor(percent)} label="Memory usage {percent.toFixed(0)}%" />
```

In `DiskPanel.svelte` (line 51):
```svelte
<ProgressBar value={percent / 100} color={getStatusColor(percent)} label="Disk usage {percent.toFixed(0)}%" />
```

**Step 3: Verify frontend builds**

Run: `cd ui && npm run build`
Expected: Builds without errors.

**Step 4: Commit**

```bash
git add ui/src/lib/components/ProgressBar.svelte ui/src/lib/components/CpuPanel.svelte ui/src/lib/components/MemoryPanel.svelte ui/src/lib/components/DiskPanel.svelte
git commit -m "fix(a11y): add aria-label prop to ProgressBar component

Screen readers now announce what each progress bar measures
(e.g., 'CPU usage 45%') instead of just 'progress bar 45'.
Resolves L5 from production readiness audit."
```

---

### Task 19: Remove Redundant IPC Call from ExportButtons [L6]

**Files:**
- Modify: `ui/src/lib/components/ExportButtons.svelte:9-55`

**Step 1: Replace `invoke('get_processes')` with `$systemSnapshot.processes`**

In `exportJSON()`, replace lines 12-13:
```typescript
// Before:
const processes = await invoke<CombinedProcess[]>('get_processes');

// After:
const processes = $systemSnapshot.processes;
```

In `exportCSV()`, replace line 48:
```typescript
// Before:
const processes = await invoke<CombinedProcess[]>('get_processes');

// After:
const processes = $systemSnapshot.processes;
```

**Step 2: Remove the unused import**

If `invoke` is no longer used in `exportJSON` or `exportCSV` for anything other than `export_to_file`, keep it. Check: `invoke` is still used for `export_to_file` on lines 35 and 61, so keep the import.

Remove the `CombinedProcess` type import if it was only used for the `invoke` generic — check: it may still be needed for the `processes` variable type. If TypeScript infers it from `$systemSnapshot.processes`, the explicit import can be removed.

**Step 3: Verify frontend builds**

Run: `cd ui && npm run build`
Expected: Builds without errors.

**Step 4: Commit**

```bash
git add ui/src/lib/components/ExportButtons.svelte
git commit -m "perf: use snapshot data instead of redundant IPC call in exports

Processes are already available in \$systemSnapshot.processes.
Removes unnecessary invoke('get_processes') call that caused
extra IPC overhead and mutex contention.
Resolves L6 from production readiness audit."
```

---

## Final Verification

### Task 20: Full Build and Test Verification

**Step 1: Run all library tests**

Run: `cargo test -p rust_dashboard_lib --verbose`
Expected: All tests pass.

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings.

**Step 3: Run fmt check**

Run: `cargo fmt -- --check`
Expected: No formatting issues.

**Step 4: Build the binary**

Run: `cargo build -p rust-dashboard --release`
Expected: Builds successfully.

**Step 5: Build the frontend**

Run: `cd ui && npm run build`
Expected: Builds successfully.

**Step 6: Update the PRODUCTION_READINESS.md checkboxes**

Mark all items as `[x] Fixed` in `PRODUCTION_READINESS.md`.

**Step 7: Final commit**

```bash
git add PRODUCTION_READINESS.md
git commit -m "docs: mark all production readiness issues as resolved"
```
