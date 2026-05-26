#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rust_dashboard_lib::config::AppConfig;
use rust_dashboard_lib::system::{CombinedProcess, ProcessDetails, SystemMonitor};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

// --- Constants ---

const HISTORY_CAPACITY: usize = 300;
const TRAY_POPUP_WIDTH: f64 = 340.0;
const TRAY_POPUP_HEIGHT: f64 = 480.0;
/// Background-thread sleep granularity. The loop sleeps in TICK-sized
/// chunks and re-reads `refresh_interval` and `paused` between each chunk,
/// so changes to either setting take effect within one tick instead of at
/// the end of the configured interval (which could be up to 60 s).
const TICK: std::time::Duration = std::time::Duration::from_millis(250);

// --- Response structs for frontend consumption ---

#[derive(serde::Serialize, Clone)]
pub struct SystemSnapshot {
    pub cpu_usage: f32,
    pub per_cpu: Vec<f32>,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub networks: Vec<NetworkInfo>,
    pub processes: Vec<CombinedProcess>,
    pub self_usage: Option<SelfUsage>,
    pub uptime_seconds: u64,
    pub load_average: (f64, f64, f64),
}

#[derive(serde::Serialize, Clone)]
pub struct SelfUsage {
    pub cpu: f32,
    pub memory: u64,
}

#[derive(serde::Serialize, Clone)]
pub struct MemoryInfo {
    pub used: u64,
    pub free: u64,
    pub total: u64,
    pub available: u64,
    pub swap_used: u64,
    pub swap_total: u64,
}

#[derive(serde::Serialize, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub filesystem: String,
    pub mount_point: String,
    pub used: u64,
    pub available: u64,
    pub total: u64,
}

#[derive(serde::Serialize, Clone)]
pub struct NetworkInfo {
    pub interface: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_rate: f64,
    pub tx_rate: f64,
}

// --- App State ---

pub struct AppState {
    pub monitor: Arc<Mutex<SystemMonitor>>,
    pub refresh_interval: Arc<AtomicU32>,
    pub paused: Arc<AtomicBool>,
    pub cpu_history: Arc<Mutex<VecDeque<(f64, f32)>>>,
    pub memory_history: Arc<Mutex<VecDeque<(f64, f64)>>>,
    pub history_start: std::time::Instant,
}

// --- Helper to build snapshot from monitor ---

fn build_snapshot(monitor: &SystemMonitor) -> SystemSnapshot {
    let (used, free, total, available, swap_used, swap_total) = monitor.memory_info();
    let per_cpu: Vec<f32> = monitor
        .sys
        .cpus()
        .iter()
        .map(|cpu| cpu.cpu_usage())
        .collect();
    let self_usage = monitor
        .usage_for_pid(std::process::id())
        .map(|(cpu, mem)| SelfUsage { cpu, memory: mem });

    SystemSnapshot {
        cpu_usage: monitor.global_cpu_usage(),
        per_cpu,
        memory: MemoryInfo {
            used,
            free,
            total,
            available,
            swap_used,
            swap_total,
        },
        disks: monitor
            .disk_info()
            .into_iter()
            .map(
                |(name, filesystem, mount_point, used, available, total)| DiskInfo {
                    name,
                    filesystem,
                    mount_point,
                    used,
                    available,
                    total,
                },
            )
            .collect(),
        networks: monitor
            .network_info_with_rates()
            .into_iter()
            .map(
                |(interface, rx_bytes, tx_bytes, rx_rate, tx_rate)| NetworkInfo {
                    interface,
                    rx_bytes,
                    tx_bytes,
                    rx_rate,
                    tx_rate,
                },
            )
            .collect(),
        processes: monitor.combined_process_list().to_vec(),
        self_usage,
        uptime_seconds: monitor.system_uptime(),
        load_average: monitor.load_average(),
    }
}

// --- Tauri Commands ---

#[tauri::command]
fn get_system_snapshot(state: tauri::State<'_, AppState>) -> Result<SystemSnapshot, String> {
    let monitor = state.monitor.lock().map_err(|e| e.to_string())?;
    Ok(build_snapshot(&monitor))
}

#[tauri::command]
fn get_processes(state: tauri::State<'_, AppState>) -> Result<Vec<CombinedProcess>, String> {
    let monitor = state.monitor.lock().map_err(|e| e.to_string())?;
    Ok(monitor.combined_process_list().to_vec())
}

#[tauri::command]
fn get_process_details(
    state: tauri::State<'_, AppState>,
    pid: u32,
) -> Result<Option<ProcessDetails>, String> {
    let monitor = state.monitor.lock().map_err(|e| e.to_string())?;
    Ok(monitor.process_details(pid))
}

#[tauri::command]
fn kill_process(state: tauri::State<'_, AppState>, pid: u32) -> Result<(), String> {
    if pid <= 1 {
        return Err("Cannot terminate system processes (PID 0 or 1)".to_string());
    }
    let mut monitor = state.monitor.lock().map_err(|e| e.to_string())?;
    monitor.kill_process(pid)
}

#[tauri::command]
fn set_refresh_interval(state: tauri::State<'_, AppState>, seconds: u32) {
    let clamped = seconds.clamp(1, 60);
    state.refresh_interval.store(clamped, Ordering::Release);
}

#[tauri::command]
fn set_paused(state: tauri::State<'_, AppState>, paused: bool) {
    state.paused.store(paused, Ordering::Release);
}

#[tauri::command]
fn manual_refresh(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut monitor = state.monitor.lock().map_err(|e| e.to_string())?;
    monitor.refresh();
    Ok(())
}

#[tauri::command]
fn get_cpu_history(state: tauri::State<'_, AppState>) -> Result<Vec<(f64, f32)>, String> {
    let history = state.cpu_history.lock().map_err(|e| e.to_string())?;
    Ok(history.iter().copied().collect())
}

#[tauri::command]
fn get_memory_history(state: tauri::State<'_, AppState>) -> Result<Vec<(f64, f64)>, String> {
    let history = state.memory_history.lock().map_err(|e| e.to_string())?;
    Ok(history.iter().copied().collect())
}

#[tauri::command]
fn load_config() -> AppConfig {
    AppConfig::load()
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())
}

#[tauri::command]
fn tray_refresh(state: tauri::State<'_, AppState>) -> Result<SystemSnapshot, String> {
    let mut monitor = state.monitor.lock().map_err(|e| e.to_string())?;
    monitor.refresh();
    Ok(build_snapshot(&monitor))
}

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

    let file_name = path_ref.file_name().ok_or("Invalid filename")?;
    let safe_path = canonical_parent.join(file_name);

    std::fs::write(&safe_path, data).map_err(|e| e.to_string())
}

// --- Main ---

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Install panic hook for crash diagnostics under panic="abort"
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {}", info);
        eprintln!("PANIC: {}", info);
    }));

    let config = AppConfig::load();
    let monitor = Arc::new(Mutex::new(SystemMonitor::new()));
    let refresh_interval = Arc::new(AtomicU32::new(config.refresh_interval_seconds));
    let paused = Arc::new(AtomicBool::new(false));
    let cpu_history = Arc::new(Mutex::new(VecDeque::with_capacity(HISTORY_CAPACITY)));
    let memory_history = Arc::new(Mutex::new(VecDeque::with_capacity(HISTORY_CAPACITY)));
    let history_start = std::time::Instant::now();

    let app_state = AppState {
        monitor: monitor.clone(),
        refresh_interval: refresh_interval.clone(),
        paused: paused.clone(),
        cpu_history: cpu_history.clone(),
        memory_history: memory_history.clone(),
        history_start,
    };

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(app_state)
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // --- Tray Icon with right-click menu ---
            let tray_handle = app_handle.clone();
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or("No default window icon found")?;

            let show_item = MenuItem::with_id(app, "show", "Show Dashboard", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("Rust Dashboard")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app_handle, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(win) = app_handle.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => {
                        app_handle.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(move |_tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        let handle = tray_handle.clone();

                        // Get physical coords from the tray icon rect
                        let (phys_x, phys_y) = match rect.position {
                            tauri::Position::Physical(p) => (p.x as f64, p.y as f64),
                            tauri::Position::Logical(p) => (p.x, p.y),
                        };
                        let (phys_w, phys_h) = match rect.size {
                            tauri::Size::Physical(s) => (s.width as f64, s.height as f64),
                            tauri::Size::Logical(s) => (s.width, s.height),
                        };

                        // Find scale factor for the monitor containing the tray icon
                        let scale = handle
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

                        // Convert to logical coordinates (consistent with inner_size)
                        let icon_x = phys_x / scale;
                        let icon_y = phys_y / scale;
                        let icon_w = phys_w / scale;
                        let icon_h = phys_h / scale;

                        // Center popup under icon, flush with menu bar bottom
                        let x = icon_x + (icon_w / 2.0) - (TRAY_POPUP_WIDTH / 2.0);
                        let y = icon_y + icon_h;

                        // Close existing popup — recreate each time to ensure
                        // correct monitor and size (avoids stale-position bugs)
                        if let Some(popup) = handle.get_webview_window("tray-popup") {
                            if popup.is_visible().unwrap_or(false) {
                                let _ = popup.close();
                                let _ = handle.emit("tray-visible", false);
                                return;
                            }
                            let _ = popup.close();
                        }

                        if let Ok(popup) = WebviewWindowBuilder::new(
                            &handle,
                            "tray-popup",
                            WebviewUrl::App("/?tray=true".into()),
                        )
                        .title("Rust Dashboard")
                        .inner_size(TRAY_POPUP_WIDTH, TRAY_POPUP_HEIGHT)
                        .position(x, y)
                        .decorations(false)
                        .shadow(false)
                        .resizable(false)
                        .always_on_top(true)
                        .visible(true)
                        .build()
                        {
                            // Ensure correct size/position after creation in case
                            // macOS placed the window on a different monitor initially
                            let _ = popup.set_size(tauri::LogicalSize::new(
                                TRAY_POPUP_WIDTH,
                                TRAY_POPUP_HEIGHT,
                            ));
                            let _ = popup.set_position(tauri::LogicalPosition::new(x, y));
                            let _ = popup.set_focus();
                            let _ = handle.emit("tray-visible", true);
                        }
                    }
                })
                .build(app)?;

            // --- Intercept main window close → hide instead ---
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        // Get the window from the event context — use the app handle
                        if let Some(win) = app_handle.get_webview_window("main") {
                            let _ = win.hide();
                        }
                    }
                });
            }

            // --- Listen for "show-main-window" event from tray popup ---
            let show_handle = app.handle().clone();
            app.listen("show-main-window", move |_event| {
                if let Some(main_win) = show_handle.get_webview_window("main") {
                    let _ = main_win.show();
                    let _ = main_win.set_focus();
                }
            });

            // --- Background thread for system monitoring ---
            let bg_handle = app.handle().clone();
            let monitor = monitor.clone();
            let refresh_interval = refresh_interval.clone();
            let paused = paused.clone();
            let cpu_history = cpu_history.clone();
            let memory_history = memory_history.clone();

            std::thread::spawn(move || {
                // Track whether we've already emitted a system-error for
                // mutex poisoning. std::sync::Mutex has no stable API to
                // clear poison, so subsequent lock() calls all return Err.
                // Emitting once per detection would cause the error banner
                // to flicker on every loop iteration after recovery. Emit
                // once on the first detection, then stay silent.
                let mut poison_alerted = false;
                loop {
                    let interval_secs = refresh_interval.load(Ordering::Acquire);
                    let was_paused = paused.load(Ordering::Acquire);

                    if !was_paused {
                        let snapshot = {
                            let mut mon = monitor.lock().unwrap_or_else(|e| {
                                log::warn!("Monitor mutex was poisoned, recovering: {}", e);
                                if !poison_alerted {
                                    let _ = bg_handle.emit(
                                        "system-error",
                                        "Monitor recovered from internal error — data may be temporarily stale",
                                    );
                                    poison_alerted = true;
                                }
                                e.into_inner()
                            });
                            mon.refresh();
                            build_snapshot(&mon)
                        };

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

                        // Emit to frontend (all windows)
                        let _ = bg_handle.emit("system-update", &snapshot);
                    }

                    // Sleep in TICK-sized chunks instead of one long sleep,
                    // so a change to refresh_interval or paused takes effect
                    // within one tick (~250 ms) rather than at the end of
                    // the configured interval (up to 60 s).
                    let target = std::time::Duration::from_secs(interval_secs as u64);
                    let mut waited = std::time::Duration::ZERO;
                    while waited < target {
                        std::thread::sleep(TICK);
                        waited += TICK;
                        if refresh_interval.load(Ordering::Acquire) != interval_secs {
                            break;
                        }
                        if paused.load(Ordering::Acquire) != was_paused {
                            break;
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_snapshot,
            get_processes,
            get_process_details,
            kill_process,
            set_refresh_interval,
            set_paused,
            manual_refresh,
            get_cpu_history,
            get_memory_history,
            load_config,
            save_config,
            export_to_file,
            tray_refresh,
        ])
        .build(tauri::generate_context!())
        .expect("error building Tauri application");

    app.run(|_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
