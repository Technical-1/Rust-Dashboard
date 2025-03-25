use std::time::Duration;
use rust_dashboard_lib::system::SystemMonitor;

#[test]
fn test_new_system() {
    let mon = SystemMonitor::new();
    // At least ensure it constructed
    assert!(!mon.sys.cpus().is_empty(), "Should have at least one CPU");
}

#[test]
fn test_refresh_no_panic() {
    let mut mon = SystemMonitor::new();
    mon.refresh();
    // No panic => success
}

#[test]
fn test_memory_info_values() {
    let mon = SystemMonitor::new();
    let (used, free, total, avail, swap_used, swap_total) = mon.memory_info();
    // We can't know exact values, but can check basic relationships
    assert!(total >= used, "Total memory >= used memory");
    assert!(total >= free, "Total memory >= free memory");
    assert!(swap_total >= swap_used, "swap total >= swap used");
    assert!(avail <= total, "available <= total memory");
}

#[test]
fn test_cpu_usage_is_reasonable() {
    let mon = SystemMonitor::new();
    let usage = mon.global_cpu_usage();
    // Usage might be 0..100+ (multi-core), just ensure it isn't negative
    assert!(usage >= 0.0, "CPU usage shouldn't be negative");
}

#[test]
fn test_disk_info_exists() {
    let mon = SystemMonitor::new();
    let _disks = mon.disk_info();
    // Just ensure it doesn't panic.
}

#[test]
fn test_network_info() {
    let mon = SystemMonitor::new();
    let nets = mon.network_info();
    // No panic => success.
    // We rename `iface` to `_iface` if not used:
    for (_iface, _rx, _tx) in nets {
        // We won't do any always-true check.
    }
}

#[test]
fn test_combined_process_list() {
    let mon = SystemMonitor::new();
    let _procs = mon.combined_process_list();
    // No panic => success
}

#[test]
fn test_usage_for_pid_current_process() {
    let mon = SystemMonitor::new();
    let current_pid = std::process::id();
    let usage = mon.usage_for_pid(current_pid);
    // No panic => success
    if let Some((cpu, _mem)) = usage {
        assert!(cpu >= 0.0, "CPU usage can't be negative");
    }
}

#[test]
fn test_multiple_refreshes() {
    let mut mon = SystemMonitor::new();
    mon.refresh();
    std::thread::sleep(Duration::from_millis(100));
    mon.refresh();
    // if we get here => success
}

#[test]
fn test_disks_not_refreshed_too_often() {
    let mut mon = SystemMonitor::new();
    let before = mon.last_disk_refresh;
    mon.refresh();
    let after = mon.last_disk_refresh;
    // They should be the same if < 60s
    assert_eq!(before, after, "Shouldn't refresh disks if <60s have elapsed");
}