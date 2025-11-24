mod config;
mod system;

use crate::config::AppConfig;
use crate::system::SystemMonitor;
use eframe::egui::{self, CentralPanel, Color32};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

/// Format bytes to human-readable string (KB, MB, GB, etc.)
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Get color based on CPU usage threshold
fn get_cpu_color(usage: f32) -> Color32 {
    if usage < 50.0 {
        Color32::from_rgb(0, 200, 0) // Green
    } else if usage < 80.0 {
        Color32::from_rgb(255, 200, 0) // Yellow
    } else {
        Color32::from_rgb(200, 0, 0) // Red
    }
}

/// Get color based on memory usage percentage
fn get_memory_color(used: u64, total: u64) -> Color32 {
    if total == 0 {
        return Color32::GRAY;
    }
    let percent = (used as f64 / total as f64) * 100.0;
    if percent < 50.0 {
        Color32::from_rgb(0, 200, 0) // Green
    } else if percent < 80.0 {
        Color32::from_rgb(255, 200, 0) // Yellow
    } else {
        Color32::from_rgb(200, 0, 0) // Red
    }
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
enum Theme {
    Light,
    Dark,
}

pub struct RustDashboardApp {
    monitor: Arc<Mutex<SystemMonitor>>,
    // Removed last_refresh and static PROCESS_REFRESH_COUNTER which were unused
    refresh_interval: Duration,
    cpu_usage: f32,
    memory_info: (u64, u64, u64, u64, u64, u64),
    disk_info: Vec<(String, String, String, u64, u64, u64)>,
    network_info: Vec<(String, u64, u64)>,
    processes: Vec<system::CombinedProcess>,
    self_usage: (f32, u64),
    refresh_interval_seconds: u32,
    refresh_interval_atomic: Arc<AtomicU32>,
    last_repaint: Instant,
    last_cpu_usage: f32,
    last_memory_info: (u64, u64, u64, u64, u64, u64),
    // Historical data for charts
    cpu_history: VecDeque<(f64, f32)>,    // (time, cpu_usage)
    memory_history: VecDeque<(f64, f64)>, // (time, memory_used_gb)
    history_start_time: Instant,
    max_history_points: usize,
    // Process search/filter
    process_search_query: String,
    process_cpu_threshold: f32,
    process_memory_threshold: u64,
    // Theme
    theme: Theme,
    // Process details expansion
    expanded_processes: std::collections::HashSet<String>,
    // Process kill confirmation
    process_to_kill: Option<(String, u32)>, // (name, pid)
    kill_confirmation_open: bool,
    // Per-CPU data
    per_cpu_usage: Vec<f32>,
    // Last refresh timestamp
    last_refresh_time: Option<Instant>,
    last_refresh_time_atomic: Arc<Mutex<Option<Instant>>>,
    // Window state for persistence
    window_size: Option<(f32, f32)>,
    window_pos: Option<(f32, f32)>,
    // Process table sort state
    process_sort_column: Option<ProcessSortColumn>,
    process_sort_ascending: bool,
    paused: bool,
    last_ui_update: Instant,
}

#[derive(Clone, Copy, PartialEq)]
enum ProcessSortColumn {
    Name,
    Cpu,
    Memory,
    Pids,
}

impl Default for RustDashboardApp {
    fn default() -> Self {
        let config = AppConfig::load();
        let refresh_interval_seconds = config.refresh_interval_seconds;
        let theme = match config.theme.as_str() {
            "Light" => Theme::Light,
            _ => Theme::Dark,
        };

        Self {
            monitor: Arc::new(Mutex::new(SystemMonitor::new())),
            refresh_interval: Duration::from_secs(refresh_interval_seconds as u64),
            cpu_usage: 0.0,
            memory_info: (0, 0, 0, 0, 0, 0),
            disk_info: Vec::new(),
            network_info: Vec::new(),
            processes: Vec::new(),
            self_usage: (0.0, 0),
            refresh_interval_seconds,
            refresh_interval_atomic: Arc::new(AtomicU32::new(refresh_interval_seconds)),
            last_repaint: Instant::now(),
            last_cpu_usage: 0.0,
            last_memory_info: (0, 0, 0, 0, 0, 0),
            cpu_history: VecDeque::with_capacity(300), // 5 minutes at 1s intervals
            memory_history: VecDeque::with_capacity(300),
            history_start_time: Instant::now(),
            max_history_points: 300,
            process_search_query: String::new(),
            process_cpu_threshold: 0.0,
            process_memory_threshold: 0,
            theme,
            expanded_processes: std::collections::HashSet::new(),
            process_to_kill: None,
            kill_confirmation_open: false,
            per_cpu_usage: Vec::new(),
            last_refresh_time: None,
            last_refresh_time_atomic: Arc::new(Mutex::new(None)),
            window_size: config
                .window_width
                .and_then(|w| config.window_height.map(|h| (w, h))),
            window_pos: config
                .window_x
                .and_then(|x| config.window_y.map(|y| (x, y))),
            process_sort_column: Some(ProcessSortColumn::Cpu),
            process_sort_ascending: false, // Descending by default (highest first)
            paused: false,
            last_ui_update: Instant::now(),
        }
    }
}

impl RustDashboardApp {
    fn export_to_json(&self) {
        use serde_json::json;
        let data = json!({
            "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            "cpu_usage": self.cpu_usage,
            "memory": {
                "used_gb": self.memory_info.0 as f64 / 1024.0 / 1024.0 / 1024.0,
                "free_gb": self.memory_info.1 as f64 / 1024.0 / 1024.0 / 1024.0,
                "total_gb": self.memory_info.2 as f64 / 1024.0 / 1024.0 / 1024.0,
            },
            "processes": self.processes.iter().map(|p| json!({
                "name": p.name,
                "cpu_usage": p.cpu_usage,
                "memory_mb": p.memory_usage / 1024 / 1024,
                "pids": p.pids
            })).collect::<Vec<_>>()
        });
        if let Ok(json_str) = serde_json::to_string_pretty(&data) {
            log::info!("Export data:\n{}", json_str);
            // In a real implementation, you'd save this to a file using a file dialog
        }
    }

    fn export_to_csv(&self) {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Write header
        if wtr
            .write_record(["Type", "Name", "CPU Usage %", "Memory MB", "PIDs"])
            .is_ok()
        {
            // Write system info
            let _ = wtr.write_record(["System", "CPU", &format!("{:.2}", self.cpu_usage), "", ""]);

            let (used_mem, _, _total_mem, _, _, _) = self.memory_info;
            let _ = wtr.write_record([
                "System",
                "Memory",
                "",
                &format!("{:.2}", used_mem as f64 / 1024.0 / 1024.0),
                "",
            ]);

            // Write processes
            for proc in &self.processes {
                let _ = wtr.write_record([
                    "Process",
                    &proc.name,
                    &format!("{:.2}", proc.cpu_usage),
                    &format!("{}", proc.memory_usage / 1024 / 1024),
                    &proc
                        .pids
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                ]);
            }

            if let Ok(data) = wtr.into_inner() {
                if let Ok(csv_str) = String::from_utf8(data) {
                    log::info!("CSV Export:\n{}", csv_str);
                    // In a real implementation, you'd save this to a file using a file dialog
                }
            }
        }
    }
}

impl eframe::App for RustDashboardApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        let mut config = AppConfig::load();
        config.refresh_interval_seconds = self.refresh_interval_seconds;
        config.theme = match self.theme {
            Theme::Light => "Light".to_string(),
            Theme::Dark => "Dark".to_string(),
        };
        if let Some((w, h)) = self.window_size {
            config.window_width = Some(w);
            config.window_height = Some(h);
        }
        if let Some((x, y)) = self.window_pos {
            config.window_x = Some(x);
            config.window_y = Some(y);
        }
        config.save().ok();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme
        let style = match self.theme {
            Theme::Dark => egui::Visuals::dark(),
            Theme::Light => egui::Visuals::light(),
        };
        ctx.set_visuals(style);

        // Set better spacing and targeted font size increases (avoiding texture atlas overflow)
        ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(12.0, 10.0); // More spacing
            style.spacing.button_padding = egui::vec2(10.0, 6.0);

            // Increase font sizes for better readability
            use egui::TextStyle;
            if let Some(font) = style.text_styles.get_mut(&TextStyle::Body) {
                font.size = 16.0; // Larger body text (was 14)
            }
            if let Some(font) = style.text_styles.get_mut(&TextStyle::Button) {
                font.size = 16.0; // Larger buttons (was 14)
            }
            if let Some(font) = style.text_styles.get_mut(&TextStyle::Monospace) {
                font.size = 15.0; // Larger monospace (was 13)
            }
        });

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Refresh Interval (s):");
                let refresh_options = [1, 2, 5, 10, 15, 30];
                egui::ComboBox::from_id_salt("refresh_combo_box")
                    .selected_text(format!("{} s", self.refresh_interval_seconds))
                    .show_ui(ui, |ui| {
                        for &val in &refresh_options {
                            ui.selectable_value(
                                &mut self.refresh_interval_seconds,
                                val,
                                format!("{} s", val),
                            );
                        }
                    });
                self.refresh_interval = Duration::from_secs(self.refresh_interval_seconds as u64);
                // Update atomic value so background thread can read it
                self.refresh_interval_atomic
                    .store(self.refresh_interval_seconds, Ordering::Relaxed);

                ui.separator();

                // Pause toggle
                ui.checkbox(&mut self.paused, "⏸ Pause Updates");

                // Manual refresh button
                if ui
                    .add_enabled(!self.paused, egui::Button::new("🔄 Refresh"))
                    .clicked()
                {
                    if let Ok(mut mon) = self.monitor.lock() {
                        mon.refresh();
                    }
                    let now = Instant::now();
                    self.last_refresh_time = Some(now);
                    if let Ok(mut time) = self.last_refresh_time_atomic.lock() {
                        *time = Some(now);
                    }
                    ctx.request_repaint();
                }

                // Update last_refresh_time from atomic (updated by background thread)
                if let Ok(time) = self.last_refresh_time_atomic.lock() {
                    if let Some(refresh_time) = *time {
                        self.last_refresh_time = Some(refresh_time);
                    }
                }

                // Show last refresh timestamp
                if let Some(refresh_time) = self.last_refresh_time {
                    let elapsed = refresh_time.elapsed();
                    let elapsed_secs = elapsed.as_secs();
                    if elapsed_secs < 60 {
                        ui.label(format!("Last refresh: {}s ago", elapsed_secs));
                    } else {
                        ui.label(format!(
                            "Last refresh: {}m {}s ago",
                            elapsed_secs / 60,
                            elapsed_secs % 60
                        ));
                    }
                } else {
                    ui.label("Last refresh: Never");
                }

                ui.separator();

                // Theme toggle
                let theme_text = match self.theme {
                    Theme::Dark => "🌙 Dark",
                    Theme::Light => "☀️ Light",
                };
                if ui.button(theme_text).clicked() {
                    self.theme = match self.theme {
                        Theme::Dark => Theme::Light,
                        Theme::Light => Theme::Dark,
                    };
                }

                ui.separator();

                let cpu = self.self_usage.0;
                let mem_gib = self.self_usage.1 as f64 / 1024.0 / 1024.0 / 1024.0;
                ui.label(format!("Dash CPU: {:.2}%", cpu));
                ui.label(format!("Dash Mem: {:.2} GiB", mem_gib));
            });
        });

        // Minimize lock duration by copying data quickly
        if !self.paused {
            let (cpu_usage, memory_info, disk_info, network_info, processes, self_usage, per_cpu) = {
                let mon = match self.monitor.lock() {
                    Ok(mon) => mon,
                    Err(e) => {
                        log::error!("Failed to acquire monitor lock: {}", e);
                        return;
                    }
                };
                // Copy all data while holding lock, then release immediately
                let per_cpu: Vec<f32> = mon.sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
                (
                    mon.global_cpu_usage(),
                    mon.memory_info(),
                    mon.disk_info(),
                    mon.network_info(),
                    mon.combined_process_list(),
                    mon.usage_for_pid(std::process::id()),
                    per_cpu,
                )
            };

            // Update fields after lock is released
            self.cpu_usage = cpu_usage;
            self.memory_info = memory_info;
            self.disk_info = disk_info;
            self.network_info = network_info;
            self.processes = processes;
            self.per_cpu_usage = per_cpu;
            if let Some((cpu, mem)) = self_usage {
                self.self_usage = (cpu, mem);
            }
        }

        // Update historical data
        if !self.paused {
            let elapsed_secs = self.history_start_time.elapsed().as_secs_f64();
            self.cpu_history.push_back((elapsed_secs, self.cpu_usage));
            let (used_mem, _, _total_mem, _, _, _) = self.memory_info;
            let used_mem_gb = used_mem as f64 / 1024.0 / 1024.0 / 1024.0;
            self.memory_history.push_back((elapsed_secs, used_mem_gb));
        }

        // Limit history size
        while self.cpu_history.len() > self.max_history_points {
            self.cpu_history.pop_front();
        }
        while self.memory_history.len() > self.max_history_points {
            self.memory_history.pop_front();
        }

        // UI Update Throttling: prevent choppy updates
        // Always request repaint to keep updating even when window is unfocused
        const MIN_UI_UPDATE_INTERVAL: Duration = Duration::from_millis(1000);

        let data_changed =
            self.cpu_usage != self.last_cpu_usage || self.memory_info != self.last_memory_info;
        let time_elapsed = self.last_repaint.elapsed() >= self.refresh_interval;
        let time_for_ui_update = self.last_ui_update.elapsed() >= MIN_UI_UPDATE_INTERVAL;

        // Always request repaint to keep updating in background
        ctx.request_repaint_after(MIN_UI_UPDATE_INTERVAL);

        if (data_changed || time_elapsed) && time_for_ui_update {
            self.last_ui_update = Instant::now();
            self.last_repaint = Instant::now();
            self.last_cpu_usage = self.cpu_usage;
            self.last_memory_info = self.memory_info;
        }

        CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2]) // Don't auto-shrink, always show scrollbar
                .show(ui, |ui| {
                ui.heading("System Stats");
                ui.add_space(10.0);

                // CPU and Memory side-by-side
                ui.columns(2, |columns| {
                    // LEFT COLUMN: CPU
                    columns[0].group(|ui| {
                        ui.heading("CPU Usage");
                        let cpu_color = get_cpu_color(self.cpu_usage);
                        let status_indicator = if self.cpu_usage < 50.0 {
                            "🟢"
                        } else if self.cpu_usage < 80.0 {
                            "🟡"
                        } else {
                            "🔴"
                        };

                        ui.horizontal(|ui| {
                            ui.label(status_indicator);
                            ui.colored_label(cpu_color, format!("{:.2}%", self.cpu_usage));
                        });
                        ui.add(egui::ProgressBar::new(self.cpu_usage / 100.0).fill(cpu_color));

                        // CPU Chart (compact)
                        if !self.cpu_history.is_empty() {
                            let points: PlotPoints = self.cpu_history.iter()
                                .map(|(t, v)| [*t, *v as f64])
                                .collect();
                            let line = Line::new(points).color(Color32::BLUE);
                            Plot::new("cpu_history")
                                .height(100.0)
                                .show_axes([false, false])
                                .show(ui, |plot_ui| {
                                    plot_ui.line(line);
                                });
                        }
                    });

                    // RIGHT COLUMN: Memory
                    columns[1].group(|ui| {
                        ui.heading("Memory");
                        let (used_mem, _free_mem, total_mem, _avail_mem, swap_used, swap_total) = self.memory_info;
                        let used_gb = used_mem as f64 / 1024.0 / 1024.0 / 1024.0;
                        let total_gb = total_mem as f64 / 1024.0 / 1024.0 / 1024.0;
                        let swap_used_gb = swap_used as f64 / 1024.0 / 1024.0 / 1024.0;
                        let swap_total_gb = swap_total as f64 / 1024.0 / 1024.0 / 1024.0;

                        let mem_color = get_memory_color(used_mem, total_mem);
                        let mem_percent = if total_mem > 0 {
                            (used_mem as f64 / total_mem as f64) * 100.0
                        } else {
                            0.0
                        };
                        let status_indicator = if mem_percent < 50.0 {
                            "🟢"
                        } else if mem_percent < 80.0 {
                            "🟡"
                        } else {
                            "🔴"
                        };

                        ui.horizontal(|ui| {
                            ui.label(status_indicator);
                            ui.colored_label(mem_color, format!("{:.2} / {:.2} GiB", used_gb, total_gb));
                        });
                        ui.add(egui::ProgressBar::new((mem_percent / 100.0) as f32).fill(mem_color));
                        ui.label(format!("Swap: {:.2} / {:.2} GiB", swap_used_gb, swap_total_gb));

                        // Memory Chart (compact)
                        if !self.memory_history.is_empty() {
                            let points: PlotPoints = self.memory_history.iter()
                                .map(|(t, v)| [*t, *v])
                                .collect();
                            let line = Line::new(points).color(Color32::GREEN);
                            Plot::new("memory_history")
                                .height(100.0)
                                .show_axes([false, false])
                                .show(ui, |plot_ui| {
                                    plot_ui.line(line);
                                });
                        }
                    });
                });

                ui.add_space(15.0);
                ui.separator();

                // Disks and Networks side-by-side
                ui.columns(2, |columns| {
                    // LEFT COLUMN: Disks
                    columns[0].group(|ui| {
                        ui.heading("Disks");
                        ui.spacing_mut().item_spacing.y = 8.0;
                        for (name, fs, mount, used, avail, total) in &self.disk_info {
                            let used_gb = *used as f64 / 1024.0 / 1024.0 / 1024.0;
                            let avail_gb = *avail as f64 / 1024.0 / 1024.0 / 1024.0;
                            let total_gb = *total as f64 / 1024.0 / 1024.0 / 1024.0;
                            ui.group(|ui| {
                                ui.label(format!("📁 {}", name));
                                ui.label(format!("FS: {} | Mount: {}", fs, mount));
                                ui.label(format!("Used: {:.2} GiB | Avail: {:.2} GiB | Total: {:.2} GiB", used_gb, avail_gb, total_gb));
                            });
                        }
                    });

                    // RIGHT COLUMN: Networks
                    columns[1].group(|ui| {
                        ui.heading("Networks");
                        ui.spacing_mut().item_spacing.y = 8.0;
                        for (iface, rx, tx) in &self.network_info {
                            ui.group(|ui| {
                                ui.label(format!("🌐 {}", iface));
                                ui.label(format!("RX: {} | TX: {}", format_bytes(*rx), format_bytes(*tx)));
                            });
                        }
                    });
                });

                ui.add_space(10.0);
                ui.separator();

                // Processes Section (always visible)
                ui.group(|ui| {
                    ui.heading("Processes");

                    // Search and filter
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        ui.text_edit_singleline(&mut self.process_search_query);
                        ui.label("CPU Threshold:");
                        ui.add(egui::Slider::new(&mut self.process_cpu_threshold, 0.0..=100.0));
                        ui.label("Memory Threshold (MB):");
                        ui.add(egui::Slider::new(&mut self.process_memory_threshold, 0..=10000));
                    });

                    // Filter processes
                    let filtered_processes: Vec<&system::CombinedProcess> = self.processes.iter()
                        .filter(|p| {
                            let matches_search = self.process_search_query.is_empty() ||
                                p.name.to_lowercase().contains(&self.process_search_query.to_lowercase());
                            let matches_cpu = p.cpu_usage >= self.process_cpu_threshold;
                            let matches_mem = (p.memory_usage / 1024 / 1024) >= self.process_memory_threshold;
                            matches_search && matches_cpu && matches_mem
                        })
                        .collect();

                    // Optimize: use indices instead of cloning entire vector
                    let mut cpu_indices: Vec<usize> = (0..filtered_processes.len()).collect();
                    cpu_indices.sort_by(|&a, &b| {
                        filtered_processes[b].cpu_usage.partial_cmp(&filtered_processes[a].cpu_usage)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    cpu_indices.truncate(5);

                    ui.separator();
                    ui.label("All Processes (Top 20):");

                    // Sort processes based on selected column
                    let mut sorted_indices: Vec<usize> = (0..filtered_processes.len()).collect();
                    if let Some(sort_col) = self.process_sort_column {
                        sorted_indices.sort_by(|&a, &b| {
                            let ordering = match sort_col {
                                ProcessSortColumn::Name => filtered_processes[a].name.cmp(&filtered_processes[b].name),
                                ProcessSortColumn::Cpu => filtered_processes[a].cpu_usage.partial_cmp(&filtered_processes[b].cpu_usage).unwrap_or(std::cmp::Ordering::Equal),
                                ProcessSortColumn::Memory => filtered_processes[a].memory_usage.cmp(&filtered_processes[b].memory_usage),
                                ProcessSortColumn::Pids => filtered_processes[a].pids.len().cmp(&filtered_processes[b].pids.len()),
                            };
                            // Stable sort tie-breaker
                            if ordering == std::cmp::Ordering::Equal {
                                filtered_processes[a].pids.first().cmp(&filtered_processes[b].pids.first())
                            } else {
                                ordering
                            }
                        });
                        if !self.process_sort_ascending {
                            sorted_indices.reverse();
                        }
                    }
                    sorted_indices.truncate(50); // Show top 50 instead of 20 for better visibility in table

                    let mut action_toggle_expand = None;
                    let mut action_kill = None;

                    // Display as sortable table using TableBuilder
                    // Wrap in ScrollArea for small windows
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::initial(200.0).at_least(100.0)) // Name
                            .column(Column::initial(80.0))  // CPU
                            .column(Column::initial(100.0)) // Memory
                            .column(Column::initial(80.0)) // PIDs (narrower)
                            .column(Column::initial(300.0))    // Actions (fixed width for scrolling)
                        .header(30.0, |mut header| {
                            header.col(|ui| {
                                if ui.selectable_label(self.process_sort_column == Some(ProcessSortColumn::Name),
                                    format!("Name {}", if self.process_sort_column == Some(ProcessSortColumn::Name) && self.process_sort_ascending { "▲" } else { "▼" })).clicked() {
                                    if self.process_sort_column == Some(ProcessSortColumn::Name) {
                                        self.process_sort_ascending = !self.process_sort_ascending;
                                    } else {
                                        self.process_sort_column = Some(ProcessSortColumn::Name);
                                        self.process_sort_ascending = true;
                                    }
                                }
                            });
                            header.col(|ui| {
                                if ui.selectable_label(self.process_sort_column == Some(ProcessSortColumn::Cpu),
                                    format!("CPU % {}", if self.process_sort_column == Some(ProcessSortColumn::Cpu) && self.process_sort_ascending { "▲" } else { "▼" })).clicked() {
                                    if self.process_sort_column == Some(ProcessSortColumn::Cpu) {
                                        self.process_sort_ascending = !self.process_sort_ascending;
                                    } else {
                                        self.process_sort_column = Some(ProcessSortColumn::Cpu);
                                        self.process_sort_ascending = false;
                                    }
                                }
                            });
                            header.col(|ui| {
                                if ui.selectable_label(self.process_sort_column == Some(ProcessSortColumn::Memory),
                                    format!("Memory MB {}", if self.process_sort_column == Some(ProcessSortColumn::Memory) && self.process_sort_ascending { "▲" } else { "▼" })).clicked() {
                                    if self.process_sort_column == Some(ProcessSortColumn::Memory) {
                                        self.process_sort_ascending = !self.process_sort_ascending;
                                    } else {
                                        self.process_sort_column = Some(ProcessSortColumn::Memory);
                                        self.process_sort_ascending = false;
                                    }
                                }
                            });
                            header.col(|ui| {
                                if ui.selectable_label(self.process_sort_column == Some(ProcessSortColumn::Pids),
                                    format!("PIDs {}", if self.process_sort_column == Some(ProcessSortColumn::Pids) && self.process_sort_ascending { "▲" } else { "▼" })).clicked() {
                                    if self.process_sort_column == Some(ProcessSortColumn::Pids) {
                                        self.process_sort_ascending = !self.process_sort_ascending;
                                    } else {
                                        self.process_sort_column = Some(ProcessSortColumn::Pids);
                                        self.process_sort_ascending = false;
                                    }
                                }
                            });
                            header.col(|ui| { ui.label("Actions"); });
                        })
                        .body(|mut body| {
                            for &idx in &sorted_indices {
                                let proc_ = filtered_processes[idx];
                                let process_key = proc_.name.to_string();
                                let is_expanded = self.expanded_processes.contains(&process_key);

                                body.row(35.0, |mut row| {
                                    row.col(|ui| {
                                        if ui.selectable_label(is_expanded, &proc_.name).clicked() {
                                            action_toggle_expand = Some(process_key.clone());
                                        }
                                    });
                                    row.col(|ui| { ui.label(format!("{:.2}", proc_.cpu_usage)); });
                                    row.col(|ui| { ui.label(format!("{}", proc_.memory_usage / 1024 / 1024)); });
                                    row.col(|ui| {
                                        // Show only first 5 PIDs
                                        let pids_display = if proc_.pids.len() <= 5 {
                                            proc_.pids.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
                                        } else {
                                            format!("{}, ... (+{})",
                                                proc_.pids.iter().take(5).map(|p| p.to_string()).collect::<Vec<_>>().join(", "),
                                                proc_.pids.len() - 5)
                                        };
                                        ui.label(pids_display);
                                    });
                                    row.col(|ui| {
                                        ui.horizontal(|ui| {
                                            for &pid in proc_.pids.iter().take(5) {
                                                if ui.button("Kill").clicked() {
                                                    action_kill = Some((proc_.name.clone(), pid));
                                                }
                                            }
                                            if proc_.pids.len() > 5 {
                                                ui.label("…");
                                            }
                                        });
                                    });
                                });

                                if is_expanded && !proc_.pids.is_empty() {
                                     body.row(80.0, |mut row| {
                                         row.col(|ui| {
                                             ui.vertical(|ui| {
                                                 ui.indent("process_details", |ui| {
                                                     if let Ok(mon) = self.monitor.lock() {
                                                         if let Some(details) = mon.process_details(proc_.pids[0]) {
                                                             ui.label(format!("Cmd: {}", details.command));
                                                             ui.label(format!("Start: {}", details.start_time));
                                                             if let Some(parent) = details.parent {
                                                                 ui.label(format!("Parent: {}", parent));
                                                             }
                                                         }
                                                     }
                                                 });
                                             });
                                         });
                                         row.col(|_| {});
                                         row.col(|_| {});
                                         row.col(|_| {});
                                         row.col(|_| {});
                                     });
                                }
                            }
                        });

                    // Apply deferred actions
                    if let Some(key) = action_toggle_expand {
                        if self.expanded_processes.contains(&key) {
                            self.expanded_processes.remove(&key);
                        } else {
                            self.expanded_processes.insert(key);
                        }
                    }
                    if let Some((name, pid)) = action_kill {
                        self.process_to_kill = Some((name, pid));
                        self.kill_confirmation_open = true;
                    }
                    }); // Close ScrollArea for table
                }); // Close group for Processes

                // Export button
                ui.add_space(10.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("📥 Export to JSON").clicked() {
                        self.export_to_json();
                    }
                    if ui.button("📥 Export to CSV").clicked() {
                        self.export_to_csv();
                    }
                });
            }); // Close ScrollArea
        }); // Close CentralPanel

        // Confirmation dialog for process kill
        if self.kill_confirmation_open {
            let (name, pid) = if let Some((ref n, p)) = self.process_to_kill {
                (n.clone(), p)
            } else {
                return;
            };

            egui::Window::new("Confirm Process Termination")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Are you sure you want to kill process:\n{} (PID: {})?",
                        name, pid
                    ));
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.kill_confirmation_open = false;
                            self.process_to_kill = None;
                        }
                        if ui.button("Kill").clicked() {
                            if let Ok(mut mon) = self.monitor.lock() {
                                if let Err(e) = mon.kill_process(pid) {
                                    log::error!("Failed to kill process {}: {}", pid, e);
                                } else {
                                    log::info!("Killed process {} ({})", pid, name);
                                }
                            }
                            self.kill_confirmation_open = false;
                            self.process_to_kill = None;
                            ctx.request_repaint();
                        }
                    });
                });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let app = RustDashboardApp::default();

    let monitor_clone = app.monitor.clone();
    let interval_atomic_clone = app.refresh_interval_atomic.clone();
    let refresh_time_clone = app.last_refresh_time_atomic.clone();
    thread::spawn(move || {
        loop {
            {
                match monitor_clone.lock() {
                    Ok(mut locked_mon) => {
                        locked_mon.refresh();
                        // Update refresh time
                        if let Ok(mut time) = refresh_time_clone.lock() {
                            *time = Some(Instant::now());
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to acquire monitor lock in background thread: {}", e);
                    }
                }
            }
            // Read refresh interval from atomic value
            let interval_seconds = interval_atomic_clone.load(Ordering::Relaxed);
            thread::sleep(Duration::from_secs(interval_seconds as u64));
        }
    });

    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Rust Dashboard",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    );
}
