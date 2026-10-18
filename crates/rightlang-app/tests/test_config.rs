use std::collections::HashMap;
use tempfile::tempdir;

use rightlang_app::config::{AppConfig, CorrectionMode};
use rightlang_app::rules::{get_frontmost_app_bundle_id, ApplicationRuleManager};

#[test]
fn test_default_config_values() {
    let config = AppConfig::default();
    assert_eq!(config.mode, CorrectionMode::Auto);
    assert_eq!(config.key_delay_us, 1000);
    assert!(config.auto_learn);
    assert!(config.notify_on_correction);
    assert!(config.notify_on_learn);
    assert!(config.hotkey_switch.is_none());
    assert!(config.hotkey_mode.is_none());

    // Verify terminal and password manager defaults are present and set to Disabled
    assert_eq!(
        config.app_rules.get("com.apple.Terminal"),
        Some(&CorrectionMode::Disabled)
    );
    assert_eq!(
        config.app_rules.get("com.googlecode.iterm2"),
        Some(&CorrectionMode::Disabled)
    );
    assert_eq!(
        config.app_rules.get("net.kovidgoyal.kitty"),
        Some(&CorrectionMode::Disabled)
    );
    assert_eq!(
        config.app_rules.get("com.agilebits.onepassword"),
        Some(&CorrectionMode::Disabled)
    );
    assert_eq!(
        config.app_rules.get("com.1password.1password"),
        Some(&CorrectionMode::Disabled)
    );
}

#[test]
fn test_toml_serialization_and_deserialization() {
    let mut custom_rules = HashMap::new();
    custom_rules.insert("com.sublimetext.4".to_string(), CorrectionMode::Manual);
    custom_rules.insert("com.tinyspeck.slackmacgap".to_string(), CorrectionMode::Auto);
    custom_rules.insert("com.apple.Terminal".to_string(), CorrectionMode::Disabled);

    let config = AppConfig {
        mode: CorrectionMode::Manual,
        key_delay_us: 2500,
        auto_learn: false,
        notify_on_correction: true,
        notify_on_learn: false,
        app_rules: custom_rules,
        hotkey_switch: Some("ctrl+alt+space".to_string()),
        hotkey_mode: Some("ctrl+alt+m".to_string()),
    };

    let toml_str = toml::to_string_pretty(&config).expect("failed to serialize config to TOML");

    // Deserialization check
    let parsed: AppConfig =
        toml::from_str(&toml_str).expect("failed to deserialize config from TOML");

    assert_eq!(parsed.mode, CorrectionMode::Manual);
    assert_eq!(parsed.key_delay_us, 2500);
    assert!(!parsed.auto_learn);
    assert!(parsed.notify_on_correction);
    assert!(!parsed.notify_on_learn);
    assert_eq!(parsed.hotkey_switch, Some("ctrl+alt+space".to_string()));
    assert_eq!(parsed.hotkey_mode, Some("ctrl+alt+m".to_string()));
    assert_eq!(
        parsed.app_rules.get("com.sublimetext.4"),
        Some(&CorrectionMode::Manual)
    );
    assert_eq!(
        parsed.app_rules.get("com.tinyspeck.slackmacgap"),
        Some(&CorrectionMode::Auto)
    );
    assert_eq!(
        parsed.app_rules.get("com.apple.Terminal"),
        Some(&CorrectionMode::Disabled)
    );
}

#[test]
fn test_partial_toml_defaults() {
    let partial_toml = r#"
mode = "disabled"
key_delay_us = 500
"#;

    let parsed: AppConfig = toml::from_str(partial_toml).expect("should parse partial TOML");
    assert_eq!(parsed.mode, CorrectionMode::Disabled);
    assert_eq!(parsed.key_delay_us, 500);
    // Missing fields should use default values
    assert!(parsed.auto_learn);
    assert!(parsed.notify_on_correction);
    assert!(parsed.notify_on_learn);
    assert!(parsed.hotkey_switch.is_none());
    assert!(parsed.hotkey_mode.is_none());
    assert_eq!(
        parsed.app_rules.get("com.apple.Terminal"),
        Some(&CorrectionMode::Disabled)
    );
}

#[test]
fn test_save_and_load_roundtrip() {
    let dir = tempdir().expect("failed to create temp dir");
    let file_path = dir.path().join("nested").join("sub").join("config.toml");

    let mut config = AppConfig {
        mode: CorrectionMode::Manual,
        key_delay_us: 1500,
        ..Default::default()
    };
    config
        .app_rules
        .insert("com.microsoft.VSCode".to_string(), CorrectionMode::Auto);

    config
        .save_to_file(&file_path)
        .expect("failed to save config");
    assert!(file_path.exists());

    let loaded = AppConfig::load_or_default(&file_path);
    assert_eq!(loaded.mode, CorrectionMode::Manual);
    assert_eq!(loaded.key_delay_us, 1500);
    assert_eq!(
        loaded.app_rules.get("com.microsoft.VSCode"),
        Some(&CorrectionMode::Auto)
    );
}

#[test]
fn test_load_or_default_fallback() {
    let non_existent = std::path::PathBuf::from("/non/existent/path/config.toml");
    let fallback = AppConfig::load_or_default(non_existent);
    assert_eq!(fallback, AppConfig::default());

    let dir = tempdir().expect("failed to create temp dir");
    let invalid_file = dir.path().join("broken.toml");
    std::fs::write(&invalid_file, "this is not valid { toml :::").unwrap();

    let fallback2 = AppConfig::load_or_default(&invalid_file);
    assert_eq!(fallback2, AppConfig::default());
}

#[test]
fn test_application_rule_manager() {
    let mut rules = HashMap::new();
    rules.insert("com.apple.Terminal".to_string(), CorrectionMode::Disabled);
    rules.insert("com.microsoft.VSCode".to_string(), CorrectionMode::Auto);

    let mut manager = ApplicationRuleManager::new(CorrectionMode::Manual, rules);

    // Explicit rules
    assert_eq!(
        manager.mode_for_app("com.apple.Terminal"),
        CorrectionMode::Disabled
    );
    assert_eq!(
        manager.mode_for_app("com.microsoft.VSCode"),
        CorrectionMode::Auto
    );

    // Unspecified app falls back to default_mode (Manual)
    assert_eq!(
        manager.mode_for_app("com.google.Chrome"),
        CorrectionMode::Manual
    );

    // Dynamically set rule
    manager.set_rule("com.google.Chrome", CorrectionMode::Auto);
    assert_eq!(
        manager.mode_for_app("com.google.Chrome"),
        CorrectionMode::Auto
    );

    // Remove rule -> falls back to default
    manager.remove_rule("com.google.Chrome");
    assert_eq!(
        manager.mode_for_app("com.google.Chrome"),
        CorrectionMode::Manual
    );

    // Change default mode
    manager.set_default_mode(CorrectionMode::Disabled);
    assert_eq!(
        manager.mode_for_app("com.google.Chrome"),
        CorrectionMode::Disabled
    );
}

#[test]
fn test_frontmost_app_detection() {
    let bundle_id = get_frontmost_app_bundle_id();
    println!("Frontmost app bundle ID in test: {:?}", bundle_id);

    #[cfg(target_os = "macos")]
    {
        // On macOS in interactive or non-headless environment, bundle_id is typically Some.
        // Even if None (headless CI), it must return without panicking.
        if let Some(ref id) = bundle_id {
            assert!(!id.is_empty(), "Bundle ID should not be empty");
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        assert!(bundle_id.is_none());
    }
}
