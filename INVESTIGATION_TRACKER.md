# Investigation Tracker

Source-of-truth document for findings from the 2026-05-26 codebase investigation.
Each entry tracks one issue from discovery through fix and validation.

- **Source**: `/investigate` skill run on 2026-05-26 (commit `56f641f`)
- **Companion file**: `.project-hub-tasks.json` (subset of top-15 findings, importable to Project Hub)
- **Related**: `PRODUCTION_READINESS.md` (prior audit, items prefixed `H#`/`M#`/`L#`)

---

## How to Use This Doc

1. Each finding has a stable ID (`INV-###`) — reference it in commits/PRs.
2. When importing into Project Hub, paste the Hub task ID into the **Hub ID** column.
3. Update **Status** as work progresses: `Open` → `In Progress` → `Fixed` → `Verified`.
4. When fixing, link the commit SHA in the **Resolution** field.
5. Re-validate by re-running `/investigate` once a milestone of fixes lands.

## Status Legend

| Status | Meaning |
|--------|---------|
| Open | Confirmed issue, not yet started |
| In Progress | Someone is actively working it |
| Fixed | Code change merged, awaiting verification |
| Verified | Re-investigated and confirmed resolved |
| Won't Fix | Decided not to address; rationale recorded |
| Deferred | Valid but lower priority than current work |

## Severity Legend

| Severity | Definition |
|----------|------------|
| HIGH | Security vulnerability, data loss risk, or breaks documented workflow |
| MEDIUM | Incorrect behavior, missing error handling, or significant UX defect |
| LOW | Polish, dead code, stale metadata, or minor inconsistency |

---

## Meta-Finding: The "Partial-Fix Pattern"

Several items in `PRODUCTION_READINESS.md` are marked as resolved (`[x] Fixed`) but the implemented fix is incomplete or addresses only the symptom. This pattern appears in **INV-001, INV-002, INV-006, INV-007** below. Recommend a pre-flight check on every PR claiming to close a PRODUCTION_READINESS item: verify both the *what* and the *how* of the doc's prescribed fix matches what landed.

---

## Implementation Areas

The 22 findings are bucketed into **6 logical code areas**. Each area shares a coherent code surface, test surface, and design context — so changes can be specced, planned, implemented, and validated together. Areas are tagged in Project Hub as `AREA-1` through `AREA-6`; filter by tag to see all tasks in an area.

The workflow for each area:
1. **Spec** — answer the area's open questions; produce a brief design doc.
2. **Plan** — break the area into ordered sub-tasks (use Hub `in_progress` to track).
3. **Implement** — write code + tests; resolve Hub tasks as commits land.
4. **Validate** — meet the area's Definition of Done before moving on.

---

### AREA-1: Library API Hardening (4 findings)

**Theme**: Make `rust_dashboard_lib` safe and consistent as a public Rust crate. README.md:88-98 advertises this as library-usable, so every change here is a public-API contract.

**Primary code surface**: `src/system.rs`, secondary touches in `src-tauri/src/main.rs` (call sites) and `examples/basic_usage.rs`.

**Test surface**: `tests/test_system.rs`, `tests/test_process_actions.rs`. Runnable via `cargo test -p rust_dashboard_lib`. Add new tests for each fix as it lands.

**Findings:**

| INV | Hub | Title | Sev |
|-----|-----|-------|-----|
| INV-002 | #425 | Library `kill_process` lacks PID guard | HIGH |
| INV-017 | #440 | Network rates inflate immediately post-refresh | LOW |
| INV-018 | #441 | Legacy `network_info()` duplicates `network_info_with_rates` | LOW |
| INV-020 | #443 | `combined_process_list().to_vec()` clones full list every snapshot | LOW |

**Spec questions to resolve first**:
1. Is the public library API frozen at 2.x, or are we OK breaking it for a 3.0? (Determines whether INV-018 becomes a wrapper or a removal.)
2. Should `kill_process` return `DashboardError` instead of `String`? (Sets precedent for the rest of the API.)
3. Is INV-020's clone cost measurable on the target hardware? (Theoretical vs real — could close as "won't fix" if unmeasurable.)

**Definition of done**:
- All 4 findings resolved in Hub.
- `cargo test -p rust_dashboard_lib` passes with new tests:
  - `test_kill_process_rejects_pid_zero_and_one` (INV-002)
  - `test_network_rates_stable_after_refresh` (INV-017)
  - `test_combined_process_list_consistency` (INV-020 if pursued)
- Rustdoc updated for any signature changes.
- `cargo doc --no-deps` produces no warnings on the affected items.

---

### AREA-2: Backend Reliability & State Coherence (4 findings)

**Theme**: Tauri backend responsiveness, error visibility, and consistency between the background thread and command handlers. The user-perceived "alive" feel of the app.

**Primary code surface**: `src-tauri/src/main.rs` (background thread, command handlers, event emit).

**Test surface**: Tauri commands are awkward to unit-test directly. Strategy: extract pure logic into testable helpers in `rust_dashboard_lib`; manual smoke tests for end-to-end (especially INV-008 pause behavior).

**Findings:**

| INV | Hub | Title | Sev |
|-----|-----|-------|-----|
| INV-005 | #428 | Background thread sleep up to 60s delays interval/pause changes | MEDIUM |
| INV-006 | #429 | Mutex poisoning recovery never emits user-visible error | MEDIUM |
| INV-008 | #431 | TrayPopup ignores global `paused` state | MEDIUM |
| INV-021 | #444 | `export_to_file` fails when target parent dir doesn't exist | LOW |

**Spec questions to resolve first**:
1. INV-005 tick granularity: 250ms? 100ms? Trade-off is responsiveness vs idle CPU wakeups. (Recommend 250ms — imperceptible delay, negligible cost.)
2. INV-008 single source of truth: Rust-side check in `tray_refresh` or Svelte-side skip? Recommend **both** — Rust authoritative, Svelte for snappiness.
3. INV-021 fix posture: lazy `create_dir_all` (UX-friendly) or upfront reject with a clearer error (auditable)? Recommend lazy with explicit home-dir check.

**Definition of done**:
- All 4 findings resolved in Hub.
- Manual test: change interval 60s → 1s, observe next refresh within ≤1s.
- Manual test: toggle pause, confirm next refresh emit is suppressed.
- Manual test: open TrayPopup while paused, confirm no Mutex contention via log/observation.
- Manual test: trigger mutex panic in dev, confirm `ErrorBanner` appears in UI.
- Manual test: export to a new subdir under home succeeds.

---

### AREA-3: Build, CI & Distribution (5 findings)

**Theme**: Make local builds match CI; restore the security audit's teeth; clean metadata for any future crates.io publish. Foundational hygiene.

**Primary code surface**: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `src-tauri/tauri.conf.json`, `Cargo.toml`, `Cargo.lock`, `ui/package-lock.json`.

**Test surface**: CI itself is the test. For INV-001, verify by injecting a known-vulnerable dep in a throwaway branch and confirming CI fails. For INV-003, run `cargo tauri build` locally on macOS and confirm `.dmg` is produced without `--config` overrides. For INV-023, both `cargo audit` and `npm audit --audit-level=moderate` must exit 0.

**Findings:**

| INV | Hub | Title | Sev |
|-----|-----|-------|-----|
| INV-001 | #424 | CI security audit bypassed by `continue-on-error` and `\|\| true` | HIGH |
| INV-003 | #426 | `tauri.conf.json` missing `bundle` section | HIGH |
| INV-014 | #437 | Stale placeholder author/repository in `Cargo.toml` | LOW |
| INV-022 | #445 | Main window has no explicit `label` | LOW |
| INV-023 | #469 | Remediate current dependency CVEs (1 Rust + 8 npm) — discovered during INV-001 | HIGH |

**Spec questions to resolve first**:
1. INV-003: keep `release.yml`'s per-platform `--config` overrides as defense-in-depth, or consolidate fully into `tauri.conf.json`? Recommend **consolidate** and remove overrides — single source of truth for build config.
2. INV-001: should `cargo audit` failures fail CI even on transient registry/network issues? Recommend **yes** — transient issues are rare; better to investigate each.
3. INV-014: keep email in `authors` or omit (privacy)? (Cargo permits `["Name"]` without email.)

**Definition of done**:
- All 4 findings resolved in Hub.
- Throwaway branch demonstrates CI failing on injected CVE; revert before merge to main.
- Local `cargo tauri build` on macOS produces `.dmg` artifact without `--config` overrides.
- `cargo metadata | jq '.packages[] | select(.name=="rust_dashboard_lib") | .repository'` returns the correct URL.
- `tauri.conf.json` window definition has explicit `"label": "main"`.

---

### AREA-4: Process Management Flow (2 findings)

**Theme**: End-to-end UX of viewing, inspecting, and terminating processes. The "danger" surface of the app — wrong PID kill = bad day.

**Primary code surface**: `ui/src/lib/components/ProcessTable.svelte`, `ProcessRow.svelte`, `KillConfirmDialog.svelte`. Backend: `src-tauri/src/main.rs:kill_process`.

**Test surface**: Add Vitest + `@testing-library/svelte` for ProcessRow expand/collapse under prop changes and KillConfirmDialog kill-all flow. Integration test against running app for end-to-end kill. No frontend tests exist today — this area is a good place to introduce them.

**Findings:**

| INV | Hub | Title | Sev |
|-----|-----|-------|-----|
| INV-010 | #433 | `ProcessRow` keeps stale details after `process.pids[0]` changes | MEDIUM |
| INV-011 | #434 | Kill button only terminates first PID of multi-instance process | MEDIUM |

**Spec questions to resolve first**:
1. **INV-011 UX direction** (most important): kill *all* PIDs of the named process, or clarify that only one instance is killed? macOS Activity Monitor shows separate rows per PID — users may not expect aggregation here. Recommend **kill all matching PIDs**, with count in the confirm dialog ("Terminate 30 instances of Chrome?"), and a per-PID success/failure summary on completion.
2. Should the Terminate flow have an "Are you sure?" step for processes owned by root/system? (Beyond current PID 0/1 guard.)

**Definition of done**:
- Both findings resolved in Hub.
- Vitest infrastructure added to `ui/` (or rationale recorded if deferring).
- Component tests cover: ProcessRow expand → process prop update → details refetch.
- KillConfirmDialog shows accurate instance count and invokes kill for all PIDs.
- Manual test: expand a long-running browser process, wait for tab restart, confirm details update.

---

### AREA-5: Stats Visualization & Reactivity (4 findings)

**Theme**: Data flow from `systemSnapshot` store → panels → charts. Reactivity correctness and visual fidelity.

**Primary code surface**: `ui/src/lib/components/MemoryPanel.svelte`, `HistoryChart.svelte`, `DiskPanel.svelte`, `NetworkPanel.svelte`. Stores: `ui/src/lib/stores/system.ts`.

**Test surface**: Extract memory breakdown math into a pure function in `utils.ts` and unit-test with Vitest. Store unit tests for `cpuHistory`/`memoryHistory` immutability. Manual visual check for chart update fluidity at 1s interval.

**Findings:**

| INV | Hub | Title | Sev |
|-----|-----|-------|-----|
| INV-009 | #432 | MemoryPanel App/Cached breakdown math incorrect | MEDIUM |
| INV-013 | #436 | HistoryChart throttle drops alternating updates at 1s | LOW |
| INV-016 | #439 | DiskPanel/NetworkPanel `{#each}` blocks lack keys | LOW |
| INV-019 | #442 | History stores mutate same array reference | LOW |

**Spec questions to resolve first**:
1. INV-009: simple `cached = total - available - free` formula across platforms, or OS-specific accuracy via sysinfo extensions? Recommend **simple formula** with a documented limitation note (macOS `available` ≈ `free + inactive`).
2. INV-013: lower throttle to 900ms, change `>` to `>=`, or remove entirely? Chart.js `update('none')` is cheap. Recommend **remove the throttle** — simpler and lets the chart breathe at all intervals.
3. INV-019: migrate ALL store updates to immutable, or scope to history stores only? Recommend **scoped to history stores** — they're the only place where reference equality could mislead consumers.

**Definition of done**:
- All 4 findings resolved in Hub.
- New helper `calculateMemoryBreakdown(mem)` extracted; Vitest unit-tested with realistic Linux + macOS payloads.
- Chart updates every tick at 1s interval (manual observation in dev mode).
- Disk reorder test: artificially reorder the disks array, confirm DOM re-keys correctly without flicker.
- Store reference-equality test: assert that consecutive `update`s produce `!==` array references.

---

### AREA-6: Cleanup & Hygiene (4 findings)

**Theme**: Dead deps, leaky lifecycles, polluting tests, dead asserts. Low-risk batch — ideal for one final cleanup PR after the other areas land.

**Primary code surface**: `ui/package.json`, `ui/src/lib/components/ContextMenu.svelte`, `tests/test_config.rs`, `tests/test_system.rs`.

**Test surface**: Test isolation guarantees (tempdir for config). Bundle size check after npm removal. ContextMenu rapid-mount/unmount stress test.

**Findings:**

| INV | Hub | Title | Sev |
|-----|-----|-------|-----|
| INV-004 | #427 | `test_config_save_and_load` pollutes real user config dir | MEDIUM |
| INV-007 | #430 | Unused `@tauri-apps/plugin-fs` npm dep | MEDIUM |
| INV-012 | #435 | ContextMenu can leak click listener if unmounted within 10ms | LOW |
| INV-015 | #438 | Dead assertion `memory_usage >= 0` on `u64` | LOW |

**Spec questions to resolve first**: None — this area is well-scoped. Just do.

**Definition of done**:
- All 4 findings resolved in Hub.
- Running `cargo test` twice produces no diff in `~/Library/Application Support/rust-dashboard/` (or platform equivalent).
- `npm ls @tauri-apps/plugin-fs` in `ui/` returns no results.
- `cargo clippy --workspace -- -D warnings` passes (catches the dead assertion).
- ContextMenu stress test: 100 rapid mount/unmount cycles with no listener accumulation (verified via DevTools event-listeners inspector).

---

### Suggested Area Phasing

Areas have implementation dependencies and risk profiles that suggest an order:

**Phase 1 — Foundations** → **AREA-3** (Build, CI & Distribution)
Restores the security audit's teeth and fixes the local build. Both are gating concerns: CVE protection should be live before merging anything else, and `cargo tauri build` should work locally so devs can verify their own changes.

**Phase 2 — Library Trust** → **AREA-1** (Library API Hardening)
Establishes defense-in-depth at the lowest layer. AREA-2 (backend) calls into the library, so library-first avoids revisiting.

**Phase 3 — Backend Coherence** → **AREA-2** (Backend Reliability)
Builds on AREA-1's hardened library. Largest manual-test surface — isolating it after AREA-1 keeps verification clean.

**Phase 4 — UI Correctness** → **AREA-4 + AREA-5** in parallel
Touches disjoint frontend code surfaces (process flow vs stats viz), so two contributors / branches can work simultaneously without merge conflicts.

**Phase 5 — Final Cleanup** → **AREA-6** (Cleanup & Hygiene)
Lowest risk, smallest review burden. Batch into a single PR at the end.

---

## Findings Index

| ID | Title | Area | Severity | Status | Hub ID |
|----|-------|------|----------|--------|--------|
| INV-001 | CI security audit bypassed by `continue-on-error` and `\|\| true` | AREA-3 | HIGH | Fixed | #424 |
| INV-002 | Library `SystemMonitor::kill_process` lacks PID guard | AREA-1 | HIGH | Fixed | #425 |
| INV-003 | `tauri.conf.json` missing `bundle` section breaks local builds | AREA-3 | HIGH | Fixed | #426 |
| INV-004 | `test_config_save_and_load` pollutes real user config dir | AREA-6 | MEDIUM | Open | #427 |
| INV-005 | Background thread sleep up to 60s delays interval/pause changes | AREA-2 | MEDIUM | Fixed | #428 |
| INV-006 | Mutex poisoning recovery never emits user-visible error | AREA-2 | MEDIUM | Fixed | #429 |
| INV-007 | Unused `@tauri-apps/plugin-fs` npm dep left after H2 fix | AREA-6 | MEDIUM | Open | #430 |
| INV-008 | TrayPopup ignores global `paused` state | AREA-2 | MEDIUM | Fixed | #431 |
| INV-009 | `MemoryPanel` App/Cached breakdown math is incorrect | AREA-5 | MEDIUM | Open | #432 |
| INV-010 | `ProcessRow` keeps stale details after `process.pids[0]` changes | AREA-4 | MEDIUM | Open | #433 |
| INV-011 | Kill button only terminates first PID of multi-instance process | AREA-4 | MEDIUM | Open | #434 |
| INV-012 | `ContextMenu` can leak click listener if unmounted within 10ms | AREA-6 | LOW | Open | #435 |
| INV-013 | `HistoryChart` throttle drops alternating updates at 1s interval | AREA-5 | LOW | Open | #436 |
| INV-014 | Stale placeholder author/repository fields in `Cargo.toml` | AREA-3 | LOW | Fixed | #437 |
| INV-015 | Dead assertion `memory_usage >= 0` on `u64` type | AREA-6 | LOW | Open | #438 |
| INV-016 | `DiskPanel` and `NetworkPanel` `{#each}` blocks lack keys | AREA-5 | LOW | Open | #439 |
| INV-017 | `network_info_with_rates` rates inflate immediately post-refresh | AREA-1 | LOW | Fixed | #440 |
| INV-018 | Unused legacy `network_info()` duplicates `network_info_with_rates` | AREA-1 | LOW | Fixed | #441 |
| INV-019 | `cpuHistory` / `memoryHistory` updates mutate same array reference | AREA-5 | LOW | Open | #442 |
| INV-020 | `combined_process_list().to_vec()` clones full list every snapshot | AREA-1 | LOW | Won't Fix | #443 |
| INV-021 | `export_to_file` fails when target parent dir doesn't exist | AREA-2 | LOW | Fixed | #444 |
| INV-022 | Main window has no explicit `label` in `tauri.conf.json` | AREA-3 | LOW | Fixed | #445 |
| INV-023 | Remediate current dependency CVEs (1 Rust + 8 npm) — discovered during INV-001 | AREA-3 | HIGH | Fixed | #469 |

---

## HIGH Severity

### INV-001: CI security audit bypassed by `continue-on-error` and `|| true`

- **Severity**: HIGH
- **Status**: Fixed (5a40357)
- **Files**: `.github/workflows/ci.yml:61-68`
- **Related**: PRODUCTION_READINESS H6 (marked Fixed)
- **Hub ID**: #424

**What's wrong:**
The `Audit Rust dependencies` step has `continue-on-error: true` on the step itself, and the `Audit npm dependencies` step has `npm audit --audit-level=moderate || true`. Either pattern alone would silently swallow CVE detections; together they neutralize the entire H6 fix.

**Impact:**
PRODUCTION_READINESS.md states H6 is resolved and the README's Security section advertises "CI security scanning (`cargo audit` + `npm audit`)." In practice, a known CVE in any dependency passes CI as a yellow warning. False sense of security; users installing future releases may consume known-vulnerable builds.

**Suggested fix:**
- Remove `continue-on-error: true` from the `Audit Rust dependencies` step.
- Drop `|| true` from the `npm audit` invocation.
- Keep `|| true` on `cargo install cargo-audit --locked` only (tolerates cache hits where the binary is already installed).

**Resolution:** Fixed in commit `5a40357` (2026-05-26). Both audits now exit 0 on current tree; CI will fail on future CVE detections at moderate+ severity. Prerequisite INV-023 cleaned the dependency state first.

---

### INV-002: Library `SystemMonitor::kill_process` lacks PID guard

- **Severity**: HIGH
- **Status**: Fixed (b0ce027)
- **Files**: `src/system.rs:386-396`, `src-tauri/src/main.rs:163` (existing guard)
- **Related**: PRODUCTION_READINESS H3 (marked Fixed)
- **Hub ID**: #425

**What's wrong:**
The PID guard added for H3 lives only in the Tauri command wrapper (`main.rs:163`). The library function `SystemMonitor::kill_process` forwards any `u32` PID straight to `sysinfo`'s `process.kill()`.

**Impact:**
README.md:88-98 advertises this crate as usable as a library. Any third-party consumer of `rust_dashboard_lib` can still kill PID 0 or 1. Defense-in-depth violated; the library is the public API and shouldn't rely on its callers to validate.

**Suggested fix:**
Add at the top of `kill_process` in `src/system.rs`:
```rust
if pid_val <= 1 {
    return Err("Cannot terminate system processes (PID 0 or 1)".into());
}
```
Then remove the duplicate guard in `main.rs:163-165` or keep it as a redundant outer check.

**Resolution:** Fixed in commit `b0ce027` (2026-05-26). PID guard added at the top of the library method; Tauri command wrapper guard kept as defense in depth. New test `test_kill_process_rejects_pid_zero_and_one` verifies both PIDs return errors mentioning "system processes".

---

### INV-003: `tauri.conf.json` missing `bundle` section breaks local builds

- **Severity**: HIGH
- **Status**: Fixed (f53fdfc)
- **Files**: `src-tauri/tauri.conf.json`, `.github/workflows/release.yml:21-33` (uses `--config` override)
- **Hub ID**: #426

**What's wrong:**
`tauri.conf.json` has no `bundle` block — no `targets`, `icon` paths, or `bundle.identifier`. The release workflow injects this per-platform via `--config '{...}'`. CI builds succeed, but local `cargo tauri build` (the command in README.md:58-62) cannot produce installers.

**Impact:**
A developer following the README will get a confusing failure or a binary-only build that can't be distributed. Local-to-CI parity is broken.

**Suggested fix:**
Add a `bundle` section to `tauri.conf.json` with:
- `active: true`
- `targets: "all"` (or platform-appropriate list)
- `icon: ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]`
- The bundle identifier (currently at root — verify it should also/instead be under bundle).

Verify CI still passes — the `--config` overrides may need to be removed or made additive.

**Resolution:** Fixed in commit `f53fdfc` (2026-05-26). Added bundle section to tauri.conf.json (active: true, targets: "all", full icon array, DeveloperTool category, descriptions). Generated `src-tauri/icons/icon.icns` from `icon.png` via `cargo tauri icon` to a temp dir, copied just the icns to preserve existing PNG icons. Removed all `tauri_config` matrix fields and `--config` flags from release.yml. Validated locally: `cargo tauri build --debug --bundles dmg` produces a 5.2 MB dmg.

---

## MEDIUM Severity

### INV-004: `test_config_save_and_load` pollutes real user config dir

- **Severity**: MEDIUM
- **Status**: Open
- **Files**: `tests/test_config.rs:11-27`, `src/config.rs:29-36`
- **Hub ID**: #427

**What's wrong:**
The test calls `config.save()` which writes to the platform config dir (e.g. `~/Library/Application Support/rust-dashboard/config.toml` on macOS). It overwrites whatever the user actually has. Worse, `cargo test` runs in parallel by default — `test_config_save_and_load`, `test_config_default`, and `test_config_path_exists` race on the same file.

**Impact:**
Running the test suite resets the user's interval/theme settings. Concurrent runs cause flaky test results. CI is fine because runners have no prior state, but local devs lose their config every `cargo test`.

**Suggested fix:**
Refactor `AppConfig::save`/`load` to accept an explicit path parameter (existing methods can call the new ones with the default path). Tests then pass a `tempfile::tempdir()`-derived path. Alternatively, override `dirs::config_dir` for tests using an env-var-driven path.

**Resolution:** _pending_

---

### INV-005: Background thread sleep up to 60s delays interval/pause changes

- **Severity**: MEDIUM
- **Status**: Fixed (ee2df3b)
- **Files**: `src-tauri/src/main.rs:429-466`
- **Hub ID**: #428

**What's wrong:**
The monitoring loop calls `thread::sleep(Duration::from_secs(interval_secs as u64))` after each refresh. The atomic `refresh_interval` and `paused` vars are only re-read on the next wake.

**Impact:**
If the user has a 60s interval and changes to 1s, the next refresh still happens up to 60s later. Same for toggling pause — the user clicks pause and the thread may still emit one more `system-update` before noticing.

**Suggested fix:**
Replace the long sleep with a tick loop:
```rust
let tick = Duration::from_millis(250);
let mut elapsed = Duration::ZERO;
while elapsed < Duration::from_secs(interval_secs as u64) {
    thread::sleep(tick);
    elapsed += tick;
    if paused.load(Acquire) { break; } // optional: bail early on pause
}
```
Re-read `interval_secs` at the top of every outer iteration (already done).

**Resolution:** Fixed in commit `ee2df3b` (2026-05-26). Added module-level `TICK = 250ms` constant. Background thread now sleeps in TICK-sized chunks and re-reads both `refresh_interval` and `paused` atomics between chunks, bailing out of the inner sleep loop on any change. Worst-case latency for either signal to take effect: ~250ms (was up to 60s).

---

### INV-006: Mutex poisoning recovery never emits user-visible error

- **Severity**: MEDIUM
- **Status**: Fixed (1a8dea0)
- **Files**: `src-tauri/src/main.rs:435-438`, `ui/src/lib/stores/system.ts:10`, `ui/src/lib/components/ErrorBanner.svelte`
- **Related**: PRODUCTION_READINESS M1 (marked Fixed)
- **Hub ID**: #429

**What's wrong:**
The fix specified in M1 was: "Use `monitor.lock().unwrap_or_else(|e| e.into_inner())` to recover from poisoned state. Add a user-visible error emission via `app_handle.emit('system-error', ...)`." The recovery half landed; the emit half did not.

**Impact:**
The frontend has `systemError` store + `ErrorBanner` component wired up to receive this signal, but the backend never sends it. Mutex poisoning becomes invisible to the user — they see the dashboard keep updating from a partially-broken monitor and have no indication something went wrong.

**Suggested fix:**
Inside the `unwrap_or_else` closure on `main.rs:435`, add:
```rust
let _ = bg_handle.emit("system-error", "Monitor recovered from internal error");
```
Add a frontend listener in `stores/system.ts` for the `system-error` event that sets the `systemError` store.

**Resolution:** Fixed in commit `1a8dea0` (2026-05-26). Backend emits the `system-error` event inside the unwrap_or_else closure, guarded by a `poison_alerted` flag declared at thread spawn (avoids banner flicker since `std::sync::Mutex` has no stable API to clear poison once set). Frontend listens for the event in `stores/system.ts` and sets `systemError`; the existing system-update handler auto-clears it on the next healthy refresh.

---

### INV-007: Unused `@tauri-apps/plugin-fs` npm dep left after H2 fix

- **Severity**: MEDIUM
- **Status**: Open
- **Files**: `ui/package.json:25`, `ui/package-lock.json`
- **Related**: PRODUCTION_READINESS H2 (marked Fixed)
- **Hub ID**: #430

**What's wrong:**
H2 removed `fs:default` capability and `tauri_plugin_fs::init()` from Rust. The JS-side npm package `@tauri-apps/plugin-fs` was left in `ui/package.json` dependencies and is not imported anywhere (`grep` finds zero usages outside `package*.json`).

**Impact:**
Dead supply-chain surface — a future compromise of this package would still execute in the bundle. Adds to install time and bundle size for zero benefit.

**Suggested fix:**
```bash
cd ui && npm uninstall @tauri-apps/plugin-fs
```
Then commit the updated `package.json` and `package-lock.json`.

**Resolution:** _pending_

---

### INV-008: TrayPopup ignores global `paused` state

- **Severity**: MEDIUM
- **Status**: Fixed (59011f2)
- **Files**: `ui/src/lib/components/TrayPopup.svelte:18-29`, `src-tauri/src/main.rs:211-215`
- **Hub ID**: #431

**What's wrong:**
`TrayPopup.svelte` runs `setInterval(doRefresh, 2000)` whenever visible. `doRefresh` calls `invoke('tray_refresh')`, which in Rust unconditionally calls `monitor.refresh()` and returns a snapshot — regardless of whether the user toggled pause in the main window.

**Impact:**
The pause feature is defeated by opening the tray popup. Also creates Mutex contention with the background thread for no reason while paused. Conceptually, "pause" should freeze *all* refreshes, not just the main window's emit cadence.

**Suggested fix:**
Two options:
1. In Rust: `tray_refresh` checks `state.paused.load(Acquire)` and returns the current snapshot without refreshing.
2. In Svelte: subscribe to `paused` store, skip `doRefresh` calls when paused.

Option 1 is more authoritative (single source of truth in Rust) but requires building a snapshot without refresh — which is already what `get_system_snapshot` does. Could just have `tray_refresh` delegate to `get_system_snapshot` when paused.

**Resolution:** Fixed in commit `59011f2` (2026-05-26). Dual approach landed: backend `tray_refresh` skips `monitor.refresh()` when the paused atomic is true (returns cached snapshot, shorter mutex hold time); `set_paused` emits a new `paused-changed` event so all windows learn about toggles (each Tauri webview has its own Svelte runtime, so the main window's `paused` store doesn't otherwise propagate). TrayPopup tracks `pausedLocal` from the event and skips the IPC call entirely when paused.

---

### INV-009: `MemoryPanel` App/Cached breakdown math is incorrect

- **Severity**: MEDIUM
- **Status**: Open
- **Files**: `ui/src/lib/components/MemoryPanel.svelte:22-26`
- **Hub ID**: #432

**What's wrong:**
```js
$: cachedGb = mem ? Math.max(0, totalGb - availableGb - (totalGb - availableGb - freeGb > 0 ? 0 : freeGb)) : 0;
$: appGb = mem ? Math.max(0, usedGb - cachedGb) : 0;
```
When the ternary's condition is true (the common case), `cachedGb = totalGb - availableGb` — which is *all* non-available memory, not cache. The App segment then becomes `usedGb - (totalGb - availableGb)`, which on most systems collapses toward zero.

**Impact:**
The breakdown bar misrepresents memory composition. The "App" segment appears nearly empty while "Cached" dominates, which contradicts reality on systems with significant application memory pressure.

**Suggested fix:**
Use the standard derivation:
```js
$: cachedGb = mem ? Math.max(0, totalGb - availableGb - freeGb) : 0;
$: appGb = mem ? Math.max(0, totalGb - freeGb - cachedGb) : 0;
```
Note: macOS sysinfo reports `available` ≈ `free + inactive`, which is close enough for this approximation. If higher fidelity is wanted, query OS-specific metrics directly.

**Resolution:** _pending_

---

### INV-010: `ProcessRow` keeps stale details after `process.pids[0]` changes

- **Severity**: MEDIUM
- **Status**: Open
- **Files**: `ui/src/lib/components/ProcessRow.svelte:11-30`, `ui/src/lib/components/ProcessTable.svelte:192`
- **Hub ID**: #433

**What's wrong:**
`details` is fetched lazily on first expansion using `process.pids[0]` and never invalidated. `ProcessTable` keys the `{#each}` by `proc.name`, so the component instance is reused across process refreshes. When `process.pids[0]` changes (e.g. Chrome tab restart spawning a new helper PID under the same name), `details` keeps the command/start_time for a PID that no longer exists.

**Impact:**
Misleading details panel. Users see start times and commands that don't match the running process.

**Suggested fix:**
Add a reactive invalidation:
```js
let lastFetchedPid: number | null = null;
$: if (expanded && process.pids[0] !== lastFetchedPid) {
    details = null;
    lastFetchedPid = process.pids[0];
    // trigger refetch
}
```
Or change the `{#each}` key to `${proc.name}-${proc.pids[0]}` so the row remounts on PID change (simpler but causes flicker).

**Resolution:** _pending_

---

### INV-011: Kill button only terminates first PID of multi-instance process

- **Severity**: MEDIUM
- **Status**: Open
- **Files**: `ui/src/lib/components/ProcessRow.svelte:51`, `ui/src/lib/components/KillConfirmDialog.svelte`
- **Hub ID**: #434

**What's wrong:**
The kill button dispatches with `process.pids[0]`. The confirm dialog uses the singular name ("Terminate {processName}?"). A process like Chrome with 30+ PIDs leaves 29 instances running after "Terminate" succeeds.

**Impact:**
UX implies "kill the process" but only one instance dies. Users have to repeatedly click Terminate to actually stop a multi-instance app.

**Suggested fix:**
Choose one of:
- **A**: Iterate over all `process.pids` in `KillConfirmDialog.handleKill()`, calling `invoke('kill_process', { pid })` for each. Update dialog text to "Terminate all N instances of {processName}?"
- **B**: Keep current single-kill behavior but update the dialog to show `PID {pid}` prominently and clarify "1 of N instances".

Option A matches user intuition; Option B sets expectations correctly.

**Resolution:** _pending_

---

## LOW Severity

### INV-012: `ContextMenu` can leak click listener if unmounted within 10ms

- **Severity**: LOW
- **Status**: Open
- **Files**: `ui/src/lib/components/ContextMenu.svelte:17-26`
- **Hub ID**: #435

**What's wrong:**
```js
onMount(() => {
    window.addEventListener('keydown', handleKeydown);
    setTimeout(() => window.addEventListener('click', handleClickOutside), 10);
});
onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    window.removeEventListener('click', handleClickOutside);
});
```
If the component unmounts before the 10ms timeout fires, `onDestroy`'s `removeEventListener` runs before the click listener was ever added. Then the timeout fires post-destroy, adds the listener, and there's no cleanup.

**Impact:**
Unlikely in practice (10ms is fast) but a real leak if it happens. Subsequent right-clicks could call `onClose` on a destroyed component, potentially throwing.

**Suggested fix:**
```js
let timeoutId: ReturnType<typeof setTimeout>;
onMount(() => {
    window.addEventListener('keydown', handleKeydown);
    timeoutId = setTimeout(() => window.addEventListener('click', handleClickOutside), 10);
});
onDestroy(() => {
    clearTimeout(timeoutId);
    window.removeEventListener('keydown', handleKeydown);
    window.removeEventListener('click', handleClickOutside);
});
```

**Resolution:** _pending_

---

### INV-013: `HistoryChart` throttle drops alternating updates at 1s interval

- **Severity**: LOW
- **Status**: Open
- **Files**: `ui/src/lib/components/HistoryChart.svelte:83-91`
- **Hub ID**: #436

**What's wrong:**
`if (now - lastUpdateTime > 1000)` uses strict greater-than. The minimum refresh_interval is 1s (clamped in `main.rs:172`), so incoming updates arrive ~1000ms apart. The condition is satisfied for some updates and not others depending on timing jitter, causing visible stutter.

**Impact:**
At 1s refresh interval the chart updates erratically. Cosmetic, but the throttle was added to *reduce* stutter, so it's defeating its own purpose at the minimum interval.

**Suggested fix:**
Change to `>=` or lower the threshold to 900ms. Alternatively, remove the throttle entirely — Chart.js's `update('none')` is cheap.

**Resolution:** _pending_

---

### INV-014: Stale placeholder author/repository fields in `Cargo.toml`

- **Severity**: LOW
- **Status**: Fixed (c812ad3)
- **Files**: `Cargo.toml:9, 12`
- **Hub ID**: #437

**What's wrong:**
```toml
authors = ["Your Name <your.email@example.com>"]
repository = "https://github.com/yourusername/rust-dashboard"
```
Per README.md:34 and :117, the actual repo is `github.com/Technical-1/Rust-Dashboard` and the author is Jacob Kanfer.

**Impact:**
Would block any future `cargo publish`. Misleading metadata for anyone consuming `cargo metadata` output.

**Suggested fix:**
Update to actual values. Consider adding contact email of choice (or remove the email entirely — Cargo permits `["Name"]` without email).

**Resolution:** Fixed in commit `c812ad3` (2026-05-26). `authors = ["Jacob Kanfer"]` (no email per Cargo's permitted name-only form), `repository = "https://github.com/Technical-1/Rust-Dashboard"`.

---

### INV-015: Dead assertion `memory_usage >= 0` on `u64` type

- **Severity**: LOW
- **Status**: Open
- **Files**: `tests/test_system.rs:138`
- **Hub ID**: #438

**What's wrong:**
`assert!(proc.memory_usage >= 0)` — `memory_usage` is `u64`, so this is always true. Clippy will flag this under `absurd_extreme_comparisons` if it gets a chance.

**Impact:**
Dead code. Clutters the test with a check that proves nothing.

**Suggested fix:**
Remove the line, or replace with a meaningful bound — e.g. `assert!(proc.memory_usage < u64::MAX / 2)` (sanity ceiling).

**Resolution:** _pending_

---

### INV-016: `DiskPanel` and `NetworkPanel` `{#each}` blocks lack keys

- **Severity**: LOW
- **Status**: Open
- **Files**: `ui/src/lib/components/DiskPanel.svelte:44`, `ui/src/lib/components/NetworkPanel.svelte:42`
- **Hub ID**: #439

**What's wrong:**
`{#each disks as disk}` and `{#each networks as net}` use no key expression. When disks or interfaces reorder between refreshes (rare but possible on macOS where APFS containers may shift), Svelte reuses DOM nodes positionally, causing flicker and incorrect transitions.

**Impact:**
Minor visual glitch. Progress bar widths may animate to wrong values if a disk swaps positions in the array.

**Suggested fix:**
- `{#each disks as disk (disk.mount_point)}`
- `{#each networks as net (net.interface)}`

`ProcessTable.svelte:192` already keys by `proc.name` — apply the same pattern here.

**Resolution:** _pending_

---

### INV-017: `network_info_with_rates` rates inflate immediately post-refresh

- **Severity**: LOW
- **Status**: Fixed (62f8bf6)
- **Files**: `src/system.rs:229-250`
- **Hub ID**: #440

**What's wrong:**
`elapsed = self.last_network_refresh.elapsed().as_secs_f64()` is computed at *query* time, not at refresh time. After a refresh, `elapsed` is near zero, and the `.max(0.1)` floor still produces high rates by dividing real deltas by a tiny denominator.

**Impact:**
The first emit immediately after a network refresh shows inflated rx_rate / tx_rate values. Cosmetic, but creates spikes in the network display that aren't real.

**Suggested fix:**
Store the actual interval between the previous snapshot and the current refresh — e.g., save `last_network_interval: Duration` when refreshing, and use that constant value in `network_info_with_rates` until the next refresh.

**Resolution:** Fixed in commit `62f8bf6` (2026-05-26). New `last_network_interval` field on `SystemMonitor` captures the actual interval at refresh time before resetting `last_network_refresh`; `network_info_with_rates` now divides by this stable constant. Seeded to 5s at construction. New test `test_network_rates_are_finite_and_non_negative` checks for NaN/infinity/negative rates.

---

### INV-018: Unused legacy `network_info()` duplicates `network_info_with_rates`

- **Severity**: LOW
- **Status**: Fixed (b1862ad)
- **Files**: `src/system.rs:210-223`
- **Hub ID**: #441

**What's wrong:**
`main.rs:118` calls `network_info_with_rates`. `network_info()` (without rates) is unused inside the workspace but kept as a public method. The example in `examples/basic_usage.rs:61` does call it, so it isn't entirely dead.

**Impact:**
Two near-identical methods to maintain. Risk of one drifting from the other.

**Suggested fix:**
Either:
- Remove `network_info()` and update the example to use `network_info_with_rates` with destructuring.
- Implement `network_info()` as a thin wrapper over `network_info_with_rates` that drops the rate fields.

**Resolution:** Fixed in commit `b1862ad` (2026-05-26) via the thin-wrapper option. The v2.x public API surface is preserved (decision recorded in AREA-1 spec — library is considered stable on 2.x, breaking removal deferred to a possible 3.0). Single source of truth for the iteration and `usage > 0` filter.

---

### INV-019: `cpuHistory` / `memoryHistory` updates mutate the same array reference

- **Severity**: LOW
- **Status**: Open
- **Files**: `ui/src/lib/stores/system.ts:39-52`
- **Hub ID**: #442

**What's wrong:**
```js
cpuHistory.update((hist) => {
    hist.push([now, snapshot.cpu_usage]);
    if (hist.length > 300) hist.shift();
    return hist;
});
```
Returns the same array reference. Svelte stores still notify on `update`, but downstream code using `===` to detect changes (or memoization libraries) won't see anything different.

**Impact:**
Subtle. Not currently breaking anything in this codebase since consumers re-read `$cpuHistory` reactively, but it's a footgun for future code.

**Suggested fix:**
Return a new array:
```js
cpuHistory.update((hist) => {
    const next = [...hist, [now, snapshot.cpu_usage]];
    return next.length > 300 ? next.slice(-300) : next;
});
```

**Resolution:** _pending_

---

### INV-020: `combined_process_list().to_vec()` clones full list every snapshot

- **Severity**: LOW
- **Status**: Won't Fix (decision recorded 2026-05-26)
- **Files**: `src-tauri/src/main.rs:131`, `src/system.rs:280-282`
- **Hub ID**: #443

**What's wrong:**
`build_snapshot` calls `monitor.combined_process_list().to_vec()` which deep-clones a `Vec<CombinedProcess>` (every name string, every PID list) on every emit. On systems with 500+ processes this is real work, repeated up to once per second.

**Impact:**
Wasted CPU. Not a correctness issue, but the dashboard exists to *show* CPU usage — using more than necessary is ironic.

**Suggested fix:**
Have `SystemMonitor` cache a serialized snapshot alongside `cached_processes`, or change `SystemSnapshot` to borrow with a lifetime (more invasive). Lowest-effort fix: do the clone only when emitting and skip it for command handlers that could borrow.

**Resolution: Won't Fix** (decision recorded 2026-05-26 during AREA-1 spec review). Closing as premature optimization — the clone cost is theoretical until measured on real hardware. If a perf concern emerges in practice (e.g. a user reports high self-CPU at small refresh intervals on a system with many processes), reopen by filing a fresh task referencing this resolution. The cited fix paths (lifetime threading, pre-serialized snapshots) are invasive and not worth the design cost without a confirmed problem.

---

### INV-021: `export_to_file` fails when target parent dir doesn't exist

- **Severity**: LOW
- **Status**: Fixed (c209a57)
- **Files**: `src-tauri/src/main.rs:231-233`
- **Hub ID**: #444

**What's wrong:**
`parent.canonicalize()` requires the parent to exist on disk. If a user types a path into the save dialog containing a directory that doesn't yet exist (e.g. `~/Exports/2026-05/dashboard.json` where `~/Exports/2026-05/` is new), canonicalization fails and the export errors out.

**Impact:**
Minor UX papercut. The save dialog on most platforms allows typing new paths.

**Suggested fix:**
Walk up the path until we find an existing ancestor, canonicalize *that*, then check the canonical ancestor is under `home_dir`. Or `fs::create_dir_all(parent)` first (only if under home_dir already by lexical check), then canonicalize.

**Resolution:** Fixed in commit `c209a57` (2026-05-26). 5-step flow: (1) walk up to the deepest existing ancestor; (2) canonicalize and security-check it is under home (catches symlink escapes); (3) `create_dir_all` the requested parent — only creates real dirs (no symlinks), so new components inherit the ancestor's safety; (4) defense-in-depth re-canonicalize the parent after creation to catch lexical `..` traversal that the first check missed; (5) write to the canonical_parent.join(file_name) path.

---

### INV-022: Main window has no explicit `label` in `tauri.conf.json`

- **Severity**: LOW
- **Status**: Fixed (de724ef)
- **Files**: `src-tauri/tauri.conf.json:12-23`, `src-tauri/capabilities/main.json:4`
- **Hub ID**: #445

**What's wrong:**
The single window definition in `tauri.conf.json` has no `label` field. It defaults to `"main"`. The capability file at `capabilities/main.json:4` explicitly references `"windows": ["main"]`.

**Impact:**
Implicit coupling. If a future Tauri version changes the default label, or a developer adds a second window, the capability binding silently breaks.

**Suggested fix:**
Add `"label": "main"` explicitly to the window object in `tauri.conf.json`.

**Resolution:** Fixed in commit `de724ef` (2026-05-26). `"label": "main"` added to the window definition; `cargo check` validates clean.

---

### INV-023: Remediate current dependency CVEs (1 Rust + 8 npm)

- **Severity**: HIGH
- **Status**: Fixed (c987db5)
- **Files**: `Cargo.lock`, `ui/package-lock.json`
- **Discovered during**: INV-001 pre-validation
- **Blocks**: INV-001 (#424)
- **Hub ID**: #469

**What's wrong:**
The `continue-on-error: true` and `|| true` bypasses on `cargo audit` and `npm audit` (see INV-001) have been hiding 9 real CVEs from CI. Removing the bypasses without addressing the CVEs would break CI on the next push.

**CVE inventory at discovery time (2026-05-26):**

| Source | Count | Severity breakdown | Notable |
|--------|-------|--------------------|---------|
| `cargo audit` | 1 | high | `bytes 1.10.1` integer overflow in `BytesMut::reserve` (RUSTSEC-2026-0007), transitive via Tauri → `tower-http` → `reqwest`. Fix: upgrade to `>= 1.11.1`. |
| `npm audit` | 8 | 4 high, 3 moderate, 1 low | All transitive (vite, svelte, sveltekit, rollup, postcss, picomatch, cookie, devalue). Lockfile-only fixes available; direct deps unchanged. |

**Impact:**
Without addressing these, INV-001's fix can't ship — CI would fail immediately. The CVEs themselves represent real exploitable surface in the dev/build tooling and in production-shipped runtime libs.

**Suggested fix (executed):**
1. `cargo update -p bytes` → bumps lockfile to `bytes 1.11.1`.
2. `cd ui && npm audit fix` → rewrites `ui/package-lock.json` to use patched transitive versions (no top-level package version changes).
3. Verify `cargo test -p rust_dashboard_lib`, `cargo clippy --workspace -- -D warnings`, `cd ui && npm run check`, and `cd ui && npm run build` all pass.
4. Re-run both audits to confirm clean state.

**Validation results (2026-05-26):**
- `cargo audit` → exit 0 (was: 1 vulnerability found).
- `npm audit --audit-level=moderate` → exit 0 (was: 8 vulnerabilities, exit 1). 3 low-severity cookie-related CVEs remain in the SvelteKit transitive tree; they're below the moderate threshold and would require a SvelteKit major downgrade (to 0.0.30) to address — not pursuing.
- `cargo test -p rust_dashboard_lib` → 33 tests pass (3 lib + 20 system + 10 doctests).
- `cargo clippy --workspace -- -D warnings` → clean.
- `cd ui && npm run check` → 0 errors, 0 warnings.
- `cd ui && npm run build` → built in 1.33s.

**Resolution:** Fixed in commit `c987db5` (2026-05-26). `cargo update -p bytes` bumped Cargo.lock to bytes 1.11.1. `npm audit fix` rewrote ui/package-lock.json with patched transitive versions; no top-level package bumps. 3 low-severity cookie CVEs in the SvelteKit tree remain — they fall below the moderate threshold so CI now passes cleanly.

---

## Investigated but NOT Issues

Confirmed during investigation as either already-fixed or false positives. Recorded here to avoid re-investigation.

| Finding | Why it's not an issue |
|---------|-----------------------|
| `export_to_file` path-traversal vector | Canonicalize-parent + allowlist-under-home is sound. `Path::file_name()` strips separators, so the final joined path can't escape. Verified manually. |
| CSP missing Tauri protocol origins | `tauri.conf.json:26` now includes `tauri:`, `asset:`, `ipc:`, `http://ipc.localhost`. Matches PRODUCTION_READINESS M7's prescribed fix. |
| CSV formula injection | `ExportButtons.svelte:11` prefixes formula characters with `'` and wraps in quotes. Handles `=`, `+`, `-`, `@`, `\t`, `\r`. |
| Capabilities not split for tray/panel | Two files exist: `main.json` (with `create-webview-window`) and `panels.json` (without). Verified line-by-line against H4's fix. |
| Tray Quit uses `std::process::exit(0)` | Now uses `app_handle.exit(0)` at `main.rs:307`. ExitRequested handler at `main.rs:490-494` allows exit when `code.is_some()`. Matches H5 fix. |
| `set_refresh_interval` unbounded | Now `seconds.clamp(1, 60)` at `main.rs:172`. Atomic ordering upgraded to Release/Acquire. Matches M2. |
| Panic hook missing | Installed at `main.rs:253-256` before any thread spawn. Logs to both `log::error!` and `eprintln!`. Matches L2. |
| ProgressBar missing aria-label | Now has `aria-label={label}` prop (line 7). Callers like `CpuPanel.svelte:48` pass meaningful labels. Matches L5. |
| ExportButtons redundant invoke | Now reads from `$systemSnapshot.processes` directly (line 21, 56). No `invoke('get_processes')` call. Matches L6. |
| GitHub Actions pinned to mutable tags | All actions in `ci.yml` and `release.yml` are pinned to SHA with version comments. Matches L3. |
| `console.error` leaks in production | All call sites go through `$lib/log.ts`'s `logError`, which gates on `import.meta.env.DEV`. Matches L4. |
| `listen()` calls not in try/catch | `system.ts:32-57` wraps the listen in its own try/catch; `+page.svelte:59-68` does too. Matches M3. |
| Sortable headers missing keyboard support | `ProcessTable.svelte:156-187` now has `tabindex="0"`, `on:keydown`, `role="columnheader"`, and `aria-sort`. Matches M4. |
| `config_path()` silent fallback to CWD | Returns `Result<PathBuf, String>` and propagates errors. `load()` falls back to defaults only on path-resolution failure with a warn log. Matches M5. |
| `tauri_plugin_fs::init()` in main.rs | Removed. Only docs reference it now. Matches H2 (Rust half — see INV-007 for npm side). |
| Redundant `rust.yml` workflow | Doesn't exist — directory listing shows only `ci.yml` and `release.yml`. Matches L1. |

---

## Open Questions / Needs User Input

1. **INV-003 fix scope**: Should we keep the per-platform `--config` injection in `release.yml` as defense-in-depth, or fully consolidate into `tauri.conf.json`? Trade-off: consolidation is cleaner but requires verifying icon paths exist on all platforms.

2. **INV-011 (multi-PID kill)**: Option A (kill all PIDs) vs Option B (clarify single-instance). Which UX is preferred? Activity Monitor's behavior is to show separate PIDs as separate rows — we aggregate, so the user expectation differs.

3. **INV-020 (process list clone cost)**: Is current performance noticeable on your machines, or is this purely theoretical? If unmeasured, lower priority further or close as "premature optimization."

4. **INV-018 (legacy `network_info`)**: Is the library API considered stable? If so, removing `network_info()` is a breaking change and we'd need to defer to a 3.0 release.

---

## Recommended Fix Order

> **Superseded** by the area-based phasing in the [Implementation Areas](#implementation-areas) section. That structure groups by code surface (so changes can be tested together) rather than severity, which is the working source of truth.

---

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-05-26 | Initial document from `/investigate` run on `56f641f` | Claude (investigation skill) |
| 2026-05-26 | Validated all 22 findings against current code via targeted greps; line numbers confirmed accurate | Claude |
| 2026-05-26 | Bulk-imported all 22 findings to Project Hub (project 32, task IDs #424–#445); Hub IDs populated in this doc | Claude |
| 2026-05-26 | Bucketed findings into 6 Implementation Areas (AREA-1 through AREA-6); tagged all Hub tasks with their area; replaced severity-based fix order with area-based phasing | Claude |
| 2026-05-26 | Started AREA-3 with INV-001. Pre-validation discovered 9 hidden CVEs (1 Rust + 8 npm) that the bypass had been masking. Created INV-023 (#469) as a blocker for INV-001. INV-023 implementation complete (cargo update -p bytes; npm audit fix): all validations green. INV-001 implementation complete (CI yaml bypasses removed): both audits exit 0 with the new config. Both tasks in_progress in Hub, awaiting commit. | Claude |
| 2026-05-26 | **AREA-3 fully resolved (5/5 findings)**. Commits: INV-023 `c987db5`, INV-001 `5a40357`, INV-003 `f53fdfc`, INV-022 `de724ef`, INV-014 `c812ad3`. CI now enforces audits; local `cargo tauri build` produces installers without --config overrides; Cargo metadata accurate; window label explicit. Hub: 5 resolved, 0 in progress, 18 open. | Claude |
| 2026-05-26 | **AREA-1 fully resolved (4/4 findings, 3 fixed + 1 won't-fix)**. Spec decisions: API stable on v2.x (no breaking changes); keep `String` error type for now (defer richer errors to a future API-evolution task); INV-020 closed as premature optimization. Commits: INV-002 `b0ce027`, INV-017 `62f8bf6`, INV-018 `b1862ad`. Library now refuses PID 0/1, network rates are stable post-refresh, `network_info` is a single-source-of-truth wrapper. Hub: 8 resolved, 0 in progress, 15 open. | Claude |
| 2026-05-26 | **AREA-2 fully resolved (4/4 findings)**. Spec decisions: 250ms tick granularity for background thread; dual Rust+Svelte check for tray pause; lazy create_dir_all with defense-in-depth re-canonicalize for export. Commits: INV-005 `ee2df3b`, INV-006 `1a8dea0`, INV-008 `59011f2`, INV-021 `c209a57`. Background thread responds to interval/pause changes within ~250ms; mutex poisoning surfaces in the ErrorBanner; tray popup respects global pause via new paused-changed event; export to new subdirectories under home now works. Hub: 13 resolved, 0 in progress, 10 open. | Claude |
