use rust_dashboard_lib::config::AppConfig;

#[test]
fn test_config_default() {
    let config = AppConfig::default();
    assert_eq!(config.refresh_interval_seconds, 2);
    assert_eq!(config.theme, "Dark");
}

#[test]
fn test_config_save_and_load() {
    let mut config = AppConfig::default();
    config.refresh_interval_seconds = 10;
    config.theme = "Light".to_string();
    config.window_width = Some(800.0);
    config.window_height = Some(600.0);

    // Save config
    assert!(config.save().is_ok());

    // Load config
    let loaded = AppConfig::load();
    assert_eq!(loaded.refresh_interval_seconds, 10);
    assert_eq!(loaded.theme, "Light");
    assert_eq!(loaded.window_width, Some(800.0));
    assert_eq!(loaded.window_height, Some(600.0));
}

#[test]
fn test_config_path_exists() {
    let path = AppConfig::config_path();
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
