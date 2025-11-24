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
fn test_terminate_process_not_available() {
    let mut mon = SystemMonitor::new();
    let pid = std::process::id();
    let result = mon.terminate_process(pid);
    // Terminate is not available, should return error
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not available"));
}

#[test]
fn test_process_details_current_process() {
    let mon = SystemMonitor::new();
    let current_pid = std::process::id();

    if let Some(details) = mon.process_details(current_pid) {
        assert_eq!(details.pid, current_pid);
        assert!(!details.name.is_empty());
        assert!(!details.command.is_empty());
        assert!(details.cpu_usage >= 0.0);
        assert!(details.memory >= 0);
    }
}

#[test]
fn test_process_details_invalid_pid() {
    let mon = SystemMonitor::new();
    let invalid_pid = u32::MAX;
    let details = mon.process_details(invalid_pid);
    assert!(details.is_none());
}
