use rust_dashboard_lib::system::SystemMonitor;

#[test]
fn test_kill_process_invalid_pid() {
    let mut mon = SystemMonitor::new();
    let invalid_pid = u32::MAX;
    let result = mon.kill_process(invalid_pid);
    // Should return error for invalid PID
    assert!(result.is_err());
}

#[test]
fn test_process_details_current_process() {
    let mon = SystemMonitor::new();
    let current_pid = std::process::id();

    if let Some(details) = mon.process_details(current_pid) {
        assert!(!details.command.is_empty());
        assert!(details.start_time > 0);
    }
}

#[test]
fn test_process_details_invalid_pid() {
    let mon = SystemMonitor::new();
    let invalid_pid = u32::MAX;
    let details = mon.process_details(invalid_pid);
    assert!(details.is_none());
}
