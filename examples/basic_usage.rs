//! Basic usage example for SystemMonitor
//!
//! This example shows how to use SystemMonitor to query system statistics.

use rust_dashboard_lib::system::SystemMonitor;
use std::thread;
use std::time::Duration;

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Create a new system monitor
    let mut monitor = SystemMonitor::new();

    println!("System Monitor Example");
    println!("======================");

    // Get CPU usage
    let cpu_usage = monitor.global_cpu_usage();
    println!("Global CPU Usage: {:.2}%", cpu_usage);

    // Get memory information
    let (used, free, total, avail, swap_used, swap_total) = monitor.memory_info();
    println!("\nMemory Information:");
    println!("  Used: {:.2} GiB", used as f64 / 1024.0 / 1024.0 / 1024.0);
    println!("  Free: {:.2} GiB", free as f64 / 1024.0 / 1024.0 / 1024.0);
    println!(
        "  Total: {:.2} GiB",
        total as f64 / 1024.0 / 1024.0 / 1024.0
    );
    println!(
        "  Available: {:.2} GiB",
        avail as f64 / 1024.0 / 1024.0 / 1024.0
    );
    println!(
        "  Swap Used: {:.2} GiB",
        swap_used as f64 / 1024.0 / 1024.0 / 1024.0
    );
    println!(
        "  Swap Total: {:.2} GiB",
        swap_total as f64 / 1024.0 / 1024.0 / 1024.0
    );

    // Get disk information
    println!("\nDisk Information:");
    let disks = monitor.disk_info();
    for (name, fs, mount, used, avail, total) in disks {
        let used_gb = used as f64 / 1024.0 / 1024.0 / 1024.0;
        let avail_gb = avail as f64 / 1024.0 / 1024.0 / 1024.0;
        let total_gb = total as f64 / 1024.0 / 1024.0 / 1024.0;
        let percent_used = (used as f64 / total as f64) * 100.0;
        println!(
            "  {} ({}) mounted at {}: {:.2}% used ({:.2} GiB / {:.2} GiB)",
            name, fs, mount, percent_used, used_gb, total_gb
        );
    }

    // Get network information
    println!("\nNetwork Information:");
    let networks = monitor.network_info();
    for (iface, rx, tx) in networks {
        let rx_mb = rx as f64 / 1024.0 / 1024.0;
        let tx_mb = tx as f64 / 1024.0 / 1024.0;
        println!("  {}: RX: {:.2} MB, TX: {:.2} MB", iface, rx_mb, tx_mb);
    }

    // Refresh and show updated CPU usage
    println!("\nRefreshing system data...");
    thread::sleep(Duration::from_millis(500));
    monitor.refresh();

    let cpu_usage_after = monitor.global_cpu_usage();
    println!("CPU Usage after refresh: {:.2}%", cpu_usage_after);

    // Get top 5 processes by CPU usage
    println!("\nTop 5 Processes by CPU Usage:");
    let mut processes = monitor.combined_process_list();
    processes.sort_by(|a, b| {
        b.cpu_usage
            .partial_cmp(&a.cpu_usage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for (i, proc) in processes.iter().take(5).enumerate() {
        println!(
            "  {}. {}: {:.2}% CPU, {} MB memory",
            i + 1,
            proc.name,
            proc.cpu_usage,
            proc.memory_usage / 1024 / 1024
        );
    }

    // Get current process information
    let current_pid = std::process::id();
    if let Some((cpu, mem)) = monitor.usage_for_pid(current_pid) {
        println!("\nCurrent Process (PID {}):", current_pid);
        println!("  CPU: {:.2}%", cpu);
        println!("  Memory: {:.2} MB", mem as f64 / 1024.0 / 1024.0);
    }

    if let Some(details) = monitor.process_details(current_pid) {
        println!("  Command: {}", details.command);
        if let Some(parent) = details.parent {
            println!("  Parent PID: {}", parent);
        }
    }
}
