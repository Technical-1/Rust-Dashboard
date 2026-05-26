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
fn test_kill_process_rejects_pid_zero_and_one() {
    let mut mon = SystemMonitor::new();
    let err0 = mon.kill_process(0).expect_err("PID 0 must be rejected");
    assert!(
        err0.contains("system processes"),
        "error should mention system processes, got: {}",
        err0
    );
    let err1 = mon.kill_process(1).expect_err("PID 1 must be rejected");
    assert!(
        err1.contains("system processes"),
        "error should mention system processes, got: {}",
        err1
    );
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
