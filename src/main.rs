mod system;

use crate::system::SystemMonitor;
use eframe::egui::{self, CentralPanel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
}

impl Default for RustDashboardApp {
    fn default() -> Self {
        Self {
            monitor: Arc::new(Mutex::new(SystemMonitor::new())),
            refresh_interval: Duration::from_secs(5),
            cpu_usage: 0.0,
            memory_info: (0, 0, 0, 0, 0, 0),
            disk_info: Vec::new(),
            network_info: Vec::new(),
            processes: Vec::new(),
            self_usage: (0.0, 0),
            refresh_interval_seconds: 5,
        }
    }
}

impl eframe::App for RustDashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Refresh Interval (s):");
                let refresh_options = [1, 2, 5, 10, 15, 30];
                egui::ComboBox::from_id_salt("refresh_combo_box")
                    .selected_text(format!("{} s", self.refresh_interval_seconds))
                    .show_ui(ui, |ui| {
                        for &val in &refresh_options {
                            ui.selectable_value(&mut self.refresh_interval_seconds, val, format!("{} s", val));
                        }
                    });
                self.refresh_interval = Duration::from_secs(self.refresh_interval_seconds as u64);

                let cpu = self.self_usage.0;
                let mem_mib = self.self_usage.1 as f64 / 1024.0 / 1024.0;
                ui.label(format!("Dash CPU: {:.2}%", cpu));
                ui.label(format!("Dash Mem: {:.2} MiB", mem_mib));
            });
        });

        {
            // lock monitor, update fields
            let mon = self.monitor.lock().unwrap();
            self.cpu_usage = mon.global_cpu_usage();
            self.memory_info = mon.memory_info();
            self.disk_info = mon.disk_info();
            self.network_info = mon.network_info();
            self.processes = mon.combined_process_list();

            if let Some((cpu, mem)) = mon.usage_for_pid(std::process::id()) {
                self.self_usage = (cpu, mem);
            }
        }

        ctx.request_repaint();

        CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("System Stats");

                ui.separator();
                ui.label(format!("CPU Usage: {:.2}%", self.cpu_usage));

                let (used_mem, free_mem, total_mem, avail_mem, swap_used, swap_total) = self.memory_info;
                ui.separator();
                ui.collapsing("Memory", |ui| {
                    let used_gb = used_mem as f64 / 1024.0 / 1024.0 / 1024.0;
                    let free_gb = free_mem as f64 / 1024.0 / 1024.0 / 1024.0;
                    let total_gb = total_mem as f64 / 1024.0 / 1024.0 / 1024.0;
                    let avail_gb = avail_mem as f64 / 1024.0 / 1024.0 / 1024.0;
                    let swap_used_gb = swap_used as f64 / 1024.0 / 1024.0 / 1024.0;
                    let swap_total_gb = swap_total as f64 / 1024.0 / 1024.0 / 1024.0;

                    ui.label(format!("Used: {:.2} GiB", used_gb));
                    ui.label(format!("Free: {:.2} GiB", free_gb));
                    ui.label(format!("Total: {:.2} GiB", total_gb));
                    ui.label(format!("Available: {:.2} GiB", avail_gb));
                    ui.label(format!("Swap Used: {:.2} GiB", swap_used_gb));
                    ui.label(format!("Swap Total: {:.2} GiB", swap_total_gb));
                });

                ui.separator();
                ui.collapsing("Disks", |ui| {
                    for (name, fs, mount, used, avail, total) in &self.disk_info {
                        let used_gb = *used as f64 / 1024.0 / 1024.0 / 1024.0;
                        let avail_gb = *avail as f64 / 1024.0 / 1024.0 / 1024.0;
                        let total_gb = *total as f64 / 1024.0 / 1024.0 / 1024.0;
                        ui.label(format!(
                            "Name: {}, FS: {}, Mount: {}, Used: {:.2} GiB, Avail: {:.2} GiB, Total: {:.2} GiB",
                            name, fs, mount, used_gb, avail_gb, total_gb
                        ));
                    }
                });

                ui.separator();
                ui.collapsing("Networks", |ui| {
                    for (iface, rx, tx) in &self.network_info {
                        ui.label(format!("Interface: {}, RX: {}, TX: {}", iface, rx, tx));
                    }
                });

                ui.separator();
                ui.collapsing("Processes", |ui| {
                    let mut processes_by_cpu = self.processes.clone();
                    processes_by_cpu.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));
                    processes_by_cpu.truncate(5);

                    ui.label("Top 5 CPU Usage:");
                    for proc_ in &processes_by_cpu {
                        ui.label(format!(
                            "Name: {}, CPU: {:.2}%, Memory: {}, PIDs: {:?}",
                            proc_.name, proc_.cpu_usage, proc_.memory_usage, proc_.pids
                        ));
                    }

                    ui.separator();
                    let mut processes_by_mem = self.processes.clone();
                    processes_by_mem.sort_by(|a, b| b.memory_usage.cmp(&a.memory_usage));
                    processes_by_mem.truncate(5);

                    ui.label("Top 5 Memory Usage:");
                    for proc_ in &processes_by_mem {
                        ui.label(format!(
                            "Name: {}, CPU: {:.2}%, Memory: {}, PIDs: {:?}",
                            proc_.name, proc_.cpu_usage, proc_.memory_usage, proc_.pids
                        ));
                    }
                });
            });
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let app = RustDashboardApp::default();

    let monitor_clone = app.monitor.clone();
    thread::spawn(move || {
        loop {
            {
                let mut locked_mon = monitor_clone.lock().unwrap();
                locked_mon.refresh();
            }
            thread::sleep(Duration::from_secs(5));
        }
    });

    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Rust Dashboard",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    );
}