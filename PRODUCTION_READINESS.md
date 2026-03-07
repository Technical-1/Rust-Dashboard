# Production Readiness Tracker

Comprehensive audit of the Rust Dashboard application.
Each issue verified against actual source code on 2026-03-07.

---

## Summary

| Severity | Count | Status |
|----------|-------|--------|
| HIGH     | 6     | All resolved |
| MEDIUM   | 7     | All resolved |
| LOW      | 6     | All resolved |
| **Total** | **19** | **All resolved** |

---

## HIGH Severity (Blockers)

### H1. Path Traversal in `export_to_file`
- **File**: `src-tauri/src/main.rs:216-231`
- **Issue**: Blocklist approach (`starts_with("/System")` etc.) is bypassable via `..` segments (e.g., `/System/../private/etc/hosts` passes the check but resolves elsewhere). Symlinks also bypass it. Provides zero protection on Windows.
- **Fix**: Canonicalize the parent directory, then allowlist only paths under `$HOME`. Use `parent.canonicalize()` before checking.
- [x] Fixed

### H2. Over-Permissive `fs:default` Capability
- **File**: `src-tauri/capabilities/default.json:19`
- **Issue**: `"fs:default"` grants full filesystem read/write to all frontend JavaScript. The export flow uses Rust's `export_to_file` command (not frontend FS APIs), so this permission is entirely unnecessary. Any XSS or supply-chain attack gains full file access.
- **Fix**: Remove `"fs:default"` from permissions. Also remove `tauri_plugin_fs::init()` from `main.rs:258` if no other code needs it.
- [x] Fixed

### H3. No PID Guard on `kill_process`
- **File**: `src-tauri/src/main.rs:162-166`, `src/system.rs:387-397`
- **Issue**: Accepts any `u32` PID with no validation. Could kill PID 0 (kernel) or PID 1 (init/launchd). The backend `system.rs:387` just passes it straight to `sysinfo::Process::kill()`.
- **Fix**: Add `if pid <= 1 { return Err("Cannot kill system processes") }` in the Tauri command.
- [x] Fixed

### H4. `create-webview-window` on All Windows
- **File**: `src-tauri/capabilities/default.json:4,14`
- **Issue**: All window patterns (`main`, `detached-*`, `tray-popup`) share the same capability set including `core:webview:allow-create-webview-window`. Secondary windows (tray, detached) should not be able to spawn new webviews — only `main` needs this (for `windowManager.ts`).
- **Fix**: Split into `capabilities/main.json` (with create-webview) and `capabilities/panels.json` (without).
- [x] Fixed

### H5. `std::process::exit(0)` Bypasses Tauri Cleanup
- **File**: `src-tauri/src/main.rs:286`
- **Issue**: Tray "Quit" menu calls `std::process::exit(0)` which skips all Tauri shutdown hooks, doesn't flush window-state plugin saves, doesn't stop the background thread, and skips all `Drop` implementations.
- **Fix**: Replace with `app_handle.exit(0)`. Also update the `RunEvent::ExitRequested` handler (line 447) to allow exit when triggered by `app.exit()` (check `code.is_some()`).
- [x] Fixed

### H6. No Security Vulnerability Scanning in CI
- **Files**: `.github/workflows/ci.yml`, `rust.yml`, `release.yml`
- **Issue**: No `cargo audit` or `npm audit` in any workflow. Known CVEs in dependencies go undetected.
- **Fix**: Add `cargo install cargo-audit && cargo audit` and `cd ui && npm audit --audit-level=moderate` steps to `ci.yml`.
- [x] Fixed

---

## MEDIUM Severity (Should Fix)

### M1. Mutex Poisoning Creates Permanent Failure Loop
- **File**: `src-tauri/src/main.rs:388-396`
- **Issue**: Once a `std::sync::Mutex` is poisoned (by a panic in any thread holding the lock), every subsequent `.lock()` returns `Err(PoisonError)`. The background thread enters an infinite error-log-sleep loop and the UI stops updating with no user-visible indication.
- **Fix**: Use `monitor.lock().unwrap_or_else(|e| e.into_inner())` to recover from poisoned state. Add a user-visible error emission via `app_handle.emit("system-error", ...)`.
- [x] Fixed

### M2. `set_refresh_interval` Has No Upper Bound
- **File**: `src-tauri/src/main.rs:169-171`
- **Issue**: `seconds.max(1)` clamps the lower bound but allows `u32::MAX` (~136 years), effectively freezing the monitor forever. Also uses `Ordering::Relaxed` for both the interval and paused atomics with no ordering guarantee between them.
- **Fix**: Change to `seconds.clamp(1, 60)`. Consider upgrading to `Ordering::Release` on store and `Ordering::Acquire` on load.
- [x] Fixed

### M3. `listen()` Calls Not Wrapped in try/catch
- **Files**: `ui/src/lib/stores/system.ts:31`, `ui/src/routes/+page.svelte:58`
- **Issue**: In `system.ts`, the `await listen('system-update', ...)` on line 31 is **outside** the try/catch block (which covers lines 15-28 only). In `+page.svelte`, the `await listen('merge-back', ...)` on line 58 is also not in a try/catch. If either fails, it's an unhandled promise rejection.
- **Fix**: Wrap both listen calls in their own try/catch blocks. Set `systemError` store on failure.
- [x] Fixed

### M4. No Keyboard Support on Sortable Table Headers
- **File**: `ui/src/lib/components/ProcessTable.svelte:149,157,165,173`
- **Issue**: Sortable `<th>` elements use `on:click` but have no `on:keydown` handler and no `tabindex`. Since `<th>` is not natively focusable, keyboard-only users cannot sort columns at all.
- **Fix**: Add `tabindex="0"` and `on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') toggleSort(...) }}` to each sortable `<th>`.
- [x] Fixed

### M5. `config_path()` Silent Fallback to Current Working Directory
- **File**: `src/config.rs:29-34`
- **Issue**: `dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))` silently falls back to CWD if the platform config directory is unavailable (sandboxed environments, containers). The `.ok()` on `create_dir_all` (line 32) also discards directory creation errors.
- **Fix**: Return `Result<PathBuf, DashboardError>` instead. Propagate errors to callers.
- [x] Fixed

### M6. CSV Export Vulnerable to Formula Injection
- **File**: `ui/src/lib/components/ExportButtons.svelte:54`
- **Issue**: CSV export only strips commas (`p.name.replace(/,/g, '')`) but doesn't escape formula injection characters (`=`, `+`, `-`, `@`). A process named `=HYPERLINK("http://evil.com","Click")` exports verbatim and executes as a formula in Excel/LibreOffice.
- **Fix**: Wrap all fields in double-quotes, escape embedded quotes, and prefix formula characters with a single quote.
- [x] Fixed

### M7. CSP Missing Tauri Protocol Origins
- **File**: `src-tauri/tauri.conf.json:26`
- **Issue**: Current CSP: `"default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self'"`. Missing explicit `img-src`, `connect-src` for `tauri://`, `asset://`, and `ipc:` protocols. The `'unsafe-inline'` in `style-src` enables style injection attacks.
- **Fix**: Expand to: `"default-src 'self' tauri: asset:; style-src 'self' 'unsafe-inline'; script-src 'self'; img-src 'self' data: blob:; connect-src ipc: http://ipc.localhost"`.
- [x] Fixed

---

## LOW Severity (Nice to Have)

### L1. Redundant `rust.yml` Workflow
- **File**: `.github/workflows/rust.yml`
- **Issue**: Nearly identical to `ci.yml` but with fewer checks (no fmt, no clippy, no matrix). Runs on same triggers (`push`/`pull_request` to `main`). Creates maintenance burden and wasted CI minutes.
- **Fix**: Delete `rust.yml` — `ci.yml` already covers everything it does and more.
- [x] Fixed

### L2. No Panic Hook Under `panic = "abort"`
- **Files**: `Cargo.toml:23`, `src-tauri/src/main.rs:235`
- **Issue**: `panic = "abort"` in release profile means any panic (including from the background thread or `sysinfo` OS API calls) immediately terminates the process with no diagnostics, no crash log, and no user feedback.
- **Fix**: Install `std::panic::set_hook()` at the top of `main()` before spawning threads. Log panic info and optionally write to a crash log file.
- [x] Fixed

### L3. GitHub Actions Pinned to Mutable Tags
- **Files**: `.github/workflows/ci.yml:38,51`, `release.yml:51`
- **Issue**: `actions/cache@v3` and `tauri-apps/tauri-action@v0` are pinned to mutable tags. A supply-chain compromise could push malicious code under the same tag, running with access to `GITHUB_TOKEN` and Apple signing secrets.
- **Fix**: Pin all actions to immutable commit SHAs with version comments: `actions/cache@<sha> # v4.1.2`.
- [x] Fixed

### L4. `console.error` Leaks Internal Details in Production
- **Files**: `ui/src/lib/stores/config.ts:17,26,35,51`, `ui/src/lib/components/ProcessRow.svelte:24`, `ui/src/lib/components/ExportButtons.svelte:38,64`, `ui/src/lib/components/TopBar.svelte:16`, `ui/src/lib/components/KillConfirmDialog.svelte:18`, `ui/src/lib/components/TrayPopup.svelte:20`
- **Issue**: 10+ locations use `console.error(e)` which dumps full Tauri error strings to DevTools. Exposes internal paths, OS error messages, and state details.
- **Fix**: Route through `systemError` store for user-facing messages. Conditionally log via `if (import.meta.env.DEV) console.error(e)`.
- [x] Fixed

### L5. `ProgressBar` Component Missing `aria-label`
- **File**: `ui/src/lib/components/ProgressBar.svelte:6`
- **Issue**: Has `role="progressbar"` and `aria-valuenow` but no `aria-label`. Screen readers announce "progress bar 45" with no context of what's being measured. (Note: `TrayPopup.svelte` progress bars already have proper `aria-label` attributes.)
- **Fix**: Add an `aria-label` prop to `ProgressBar.svelte` and pass context from each usage (e.g., "CPU usage", "Memory usage").
- [x] Fixed

### L6. ExportButtons Makes Redundant IPC Call
- **File**: `ui/src/lib/components/ExportButtons.svelte:13,48`
- **Issue**: Both `exportJSON()` and `exportCSV()` call `invoke('get_processes')` even though processes are already available in `$systemSnapshot.processes`. This creates unnecessary IPC overhead and mutex contention.
- **Fix**: Use `$systemSnapshot.processes` directly instead of a separate invoke call.
- [x] Fixed

---

## Validated as NOT Issues (Removed from Tracking)

These were flagged by automated analysis but confirmed as false positives after manual code review:

| Original Finding | Why It's Not an Issue |
|-----|-----|
| HistoryChart missing null check | Line 83 already has `if (chart && data.length > 0)` guard |
| `+layout.svelte` listen('merge-back') unguarded | Wrong file — it's in `+page.svelte:58`. Root layout is 28 lines of theme logic only |
| TrayPopup reduce recalculated every render | It's a Svelte `$:` reactive statement — only runs when `snapshot` changes |
| Chart.register in module scope | `Chart.register()` is idempotent by design |
| ContextMenu 10ms setTimeout fragile | Standard pattern to prevent opening click from closing the menu |
| DetachedHeader a11y on mousedown | Window drag handle — mouse-driven by necessity (Tauri `startDragging()`) |
| TrayPopup progress bars missing aria-labels | Lines 120, 134, 148 already have `aria-label="CPU usage"` etc. |

---

## Recommended Fix Order

**Phase 1 — Security (H1-H4, H6, M7)**: Fix all security issues first. These are the blockers.

**Phase 2 — Stability (H5, M1, M2, M3)**: Fix exit handling, mutex recovery, and error propagation. Prevents silent failures in production.

**Phase 3 — Quality (M4-M6, L1-L6)**: Accessibility, CSV safety, config robustness, CI cleanup, and polish.
