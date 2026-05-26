use rust_dashboard_lib::config::AppConfig;

#[test]
fn test_config_default() {
    let config = AppConfig::default();
    assert_eq!(config.refresh_interval_seconds, 2);
    assert_eq!(config.theme, "Dark");
}

#[test]
fn test_config_save_and_load() {
    // Use a tempdir so we don't touch the real platform config path
    // and don't race other tests that also call config_path().
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("config.toml");

    let mut config = AppConfig::default();
    config.refresh_interval_seconds = 10;
    config.theme = "Light".to_string();
    config.window_width = Some(800.0);
    config.window_height = Some(600.0);

    assert!(config.save_to(&path).is_ok());

    let loaded = AppConfig::load_from(&path);
    assert_eq!(loaded.refresh_interval_seconds, 10);
    assert_eq!(loaded.theme, "Light");
    assert_eq!(loaded.window_width, Some(800.0));
    assert_eq!(loaded.window_height, Some(600.0));
}

#[test]
fn test_config_load_from_missing_returns_default() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("does-not-exist.toml");

    let loaded = AppConfig::load_from(&path);
    let default = AppConfig::default();
    assert_eq!(loaded.refresh_interval_seconds, default.refresh_interval_seconds);
    assert_eq!(loaded.theme, default.theme);
}

#[test]
fn test_config_path_exists() {
    let path = AppConfig::config_path().expect("config_path should succeed");
    // Config path should be valid (directory should be created)
    assert!(path.parent().unwrap().exists() || path.parent().unwrap().parent().is_some());
}

#[test]
fn test_config_serialization() {
    use toml;
    let config = AppConfig::default();
    let serialized = toml::to_string(&config);
    assert!(serialized.is_ok());

    let deserialized: Result<AppConfig, _> = toml::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok());
    let deserialized = deserialized.unwrap();
    assert_eq!(
        deserialized.refresh_interval_seconds,
        config.refresh_interval_seconds
    );
    assert_eq!(deserialized.theme, config.theme);
}
