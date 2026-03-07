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
const TRAY_Y_OFFSET: f64 = 10.0;

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
    let clamped = seconds.max(1);
    state.refresh_interval.store(clamped, Ordering::Relaxed);
}

#[tauri::command]
fn set_paused(state: tauri::State<'_, AppState>, paused: bool) {
    state.paused.store(paused, Ordering::Relaxed);
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

    let file_name = path_ref
        .file_name()
        .ok_or("Invalid filename")?;
    let safe_path = canonical_parent.join(file_name);

    std::fs::write(&safe_path, data).map_err(|e| e.to_string())
}

// --- Main ---

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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
            let icon = app.default_window_icon().cloned().ok_or("No default window icon found")?;

            let show_item = MenuItem::with_id(app, "show", "Show Dashboard", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("Rust Dashboard")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app_handle, event| {
                    match event.id.as_ref() {
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
                        let handle = tray_handle.clone();

                        // Get scale factor for coordinate conversion
                        let scale = handle
                            .get_webview_window("main")
                            .and_then(|w| w.scale_factor().ok())
                            .unwrap_or(2.0);

                        // Convert tray icon rect to logical coordinates
                        let (icon_x, icon_y) = match rect.position {
                            tauri::Position::Physical(p) => (p.x as f64 / scale, p.y as f64 / scale),
                            tauri::Position::Logical(p) => (p.x, p.y),
                        };
                        let (icon_w, icon_h) = match rect.size {
                            tauri::Size::Physical(s) => (s.width as f64 / scale, s.height as f64 / scale),
                            tauri::Size::Logical(s) => (s.width, s.height),
                        };

                        let x = icon_x + (icon_w / 2.0) - (TRAY_POPUP_WIDTH / 2.0);
                        let y = icon_y + icon_h - TRAY_Y_OFFSET;

                        // Toggle: if popup exists, show/hide it; otherwise create it once
                        if let Some(popup) = handle.get_webview_window("tray-popup") {
                            if popup.is_visible().unwrap_or(false) {
                                let _ = popup.hide();
                                let _ = handle.emit("tray-visible", false);
                            } else {
                                let _ = popup.set_position(tauri::LogicalPosition::new(x, y));
                                let _ = popup.show();
                                let _ = popup.set_focus();
                                let _ = handle.emit("tray-visible", true);
                            }
                        } else {
                            let _ = WebviewWindowBuilder::new(
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
                            .build();
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
                loop {
                    let interval_secs = refresh_interval.load(Ordering::Relaxed);

                    if !paused.load(Ordering::Relaxed) {
                        let snapshot = {
                            let mut mon = match monitor.lock() {
                                Ok(m) => m,
                                Err(e) => {
                                    log::error!("Failed to lock monitor: {}", e);
                                    std::thread::sleep(std::time::Duration::from_secs(
                                        interval_secs as u64,
                                    ));
                                    continue;
                                }
                            };
                            mon.refresh();
                            build_snapshot(&mon)
                        };

                        // Update history
                        let elapsed = history_start.elapsed().as_secs_f64();
                        if let Ok(mut hist) = cpu_history.lock() {
                            hist.push_back((elapsed, snapshot.cpu_usage));
                            while hist.len() > HISTORY_CAPACITY {
                                hist.pop_front();
                            }
                        }
                        if let Ok(mut hist) = memory_history.lock() {
                            let used_gb = snapshot.memory.used as f64 / 1024.0 / 1024.0 / 1024.0;
                            hist.push_back((elapsed, used_gb));
                            while hist.len() > HISTORY_CAPACITY {
                                hist.pop_front();
                            }
                        }

                        // Emit to frontend (all windows)
                        let _ = bg_handle.emit("system-update", &snapshot);
                    }

                    std::thread::sleep(std::time::Duration::from_secs(interval_secs as u64));
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
