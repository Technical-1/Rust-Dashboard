use rust_dashboard_lib::system::SystemMonitor;
use std::time::Duration;

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
    assert_eq!(
        before, after,
        "Shouldn't refresh disks if <60s have elapsed"
    );
}

#[test]
fn test_usage_for_invalid_pid() {
    let mon = SystemMonitor::new();
    // Use a very large PID that likely doesn't exist
    let invalid_pid = u32::MAX;
    let usage = mon.usage_for_pid(invalid_pid);
    assert!(usage.is_none(), "Invalid PID should return None");
}

#[test]
fn test_process_details_invalid_pid() {
    let mon = SystemMonitor::new();
    let invalid_pid = u32::MAX;
    let details = mon.process_details(invalid_pid);
    assert!(details.is_none(), "Invalid PID should return None");
}

#[test]
fn test_process_details_current_process() {
    let mon = SystemMonitor::new();
    let current_pid = std::process::id();
    if let Some(details) = mon.process_details(current_pid) {
        assert_eq!(details.pid, current_pid);
        assert!(!details.name.is_empty());
        assert!(details.cpu_usage >= 0.0);
        assert!(details.memory >= 0);
    }
}

#[test]
fn test_combined_process_list_not_empty() {
    let mon = SystemMonitor::new();
    let processes = mon.combined_process_list();
    // Should have at least the current process
    assert!(!processes.is_empty(), "Should have at least one process");
}

#[test]
fn test_combined_process_list_aggregation() {
    let mon = SystemMonitor::new();
    let processes = mon.combined_process_list();
    // Check that processes are properly aggregated
    for proc in processes {
        assert!(!proc.name.is_empty());
        assert!(proc.cpu_usage >= 0.0);
        assert!(proc.memory_usage >= 0);
        assert!(!proc.pids.is_empty());
    }
}

#[test]
fn test_concurrent_refresh() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let monitor = Arc::new(Mutex::new(SystemMonitor::new()));
    let mut handles = vec![];

    // Spawn multiple threads that refresh concurrently
    for _ in 0..5 {
        let monitor_clone = monitor.clone();
        let handle = thread::spawn(move || {
            for _ in 0..10 {
                if let Ok(mut mon) = monitor_clone.lock() {
                    mon.refresh();
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify monitor is still usable
    let mon = monitor.lock().unwrap();
    let cpu = mon.global_cpu_usage();
    assert!(cpu >= 0.0);
}

#[test]
fn test_disk_info_format() {
    let mon = SystemMonitor::new();
    let disks = mon.disk_info();
    for (name, fs, mount, used, avail, total) in disks {
        assert!(!name.is_empty());
        assert!(!mount.is_empty());
        assert!(total >= used);
        assert!(total >= avail);
        assert_eq!(used + avail, total);
    }
}

#[test]
fn test_network_info_format() {
    let mon = SystemMonitor::new();
    let networks = mon.network_info();
    for (iface, rx, tx) in networks {
        assert!(!iface.is_empty());
        assert!(rx >= 0);
        assert!(tx >= 0);
    }
}

#[test]
fn test_cpu_usage_consistency() {
    let mut mon = SystemMonitor::new();
    let cpu1 = mon.global_cpu_usage();
    std::thread::sleep(Duration::from_millis(100));
    mon.refresh();
    let cpu2 = mon.global_cpu_usage();
    // CPU usage should be reasonable (not negative, not impossibly high)
    assert!(
        cpu1 >= 0.0 && cpu1 <= 1000.0,
        "CPU usage should be reasonable"
    );
    assert!(
        cpu2 >= 0.0 && cpu2 <= 1000.0,
        "CPU usage should be reasonable"
    );
}

#[test]
fn test_memory_consistency_after_refresh() {
    let mut mon = SystemMonitor::new();
    let (used1, free1, total1, _, _, _) = mon.memory_info();
    mon.refresh();
    let (used2, free2, total2, _, _, _) = mon.memory_info();

    // Total memory should remain constant
    assert_eq!(total1, total2, "Total memory should not change");
    // Used + free should equal total (approximately, due to rounding)
    assert!(
        (used1 + free1) <= total1 + 1024,
        "Used + free should <= total"
    );
    assert!(
        (used2 + free2) <= total2 + 1024,
        "Used + free should <= total"
    );
}
