use rust_dashboard_lib::system::CombinedProcess;

#[test]
fn test_csv_export_format() {
    use csv::Writer;

    let mut wtr = Writer::from_writer(vec![]);

    // Write test data
    assert!(wtr
        .write_record(&["Type", "Name", "CPU Usage %", "Memory MB", "PIDs"])
        .is_ok());
    assert!(wtr
        .write_record(&["Process", "test", "10.5", "100", "1234"])
        .is_ok());

    let data = wtr.into_inner().unwrap();
    let csv_str = String::from_utf8(data).unwrap();

    assert!(csv_str.contains("Type"));
    assert!(csv_str.contains("test"));
    assert!(csv_str.contains("10.5"));
}

#[test]
fn test_json_export_structure() {
    use serde_json::json;

    let data = json!({
        "timestamp": 1234567890,
        "cpu_usage": 50.5,
        "memory": {
            "used_gb": 4.0,
            "free_gb": 4.0,
            "total_gb": 8.0
        },
        "processes": []
    });

    let json_str = serde_json::to_string_pretty(&data).unwrap();
    assert!(json_str.contains("cpu_usage"));
    assert!(json_str.contains("memory"));
    assert!(json_str.contains("processes"));
}

#[test]
fn test_process_serialization() {
    use serde_json;

    let proc = CombinedProcess {
        name: "test_process".to_string(),
        cpu_usage: 25.5,
        memory_usage: 1024 * 1024 * 100, // 100 MB
        pids: vec![1234, 5678],
    };

    let json = serde_json::json!({
        "name": proc.name,
        "cpu_usage": proc.cpu_usage,
        "memory_mb": proc.memory_usage / 1024 / 1024,
        "pids": proc.pids
    });

    assert_eq!(json["name"], "test_process");
    assert_eq!(json["cpu_usage"], 25.5);
    assert_eq!(json["memory_mb"], 100);
    assert_eq!(
        json["pids"].as_array().unwrap(),
        &vec![serde_json::json!(1234), serde_json::json!(5678)]
    );
}
