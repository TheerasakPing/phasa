use rightlang_app::config::CorrectionMode;
use rightlang_app::notify::NotificationManager;
use rightlang_app::tray::{TrayAction, TrayManager};

#[test]
fn test_notification_manager_basic_lifecycle() {
    let mut manager = NotificationManager::new(true);
    assert!(manager.is_enabled());

    // Initially no notification recorded
    assert!(manager.last_notification().is_none());

    // Send a correction notification
    let sent = manager.notify_correction("8iy[", "ครับ ");
    assert!(sent);

    let (title, body) = manager
        .last_notification()
        .expect("Notification must be recorded");
    assert_eq!(title, "RightLang");
    assert_eq!(body, "8iy[ → ครับ");

    // Clear recorded notification
    manager.clear_last_notification();
    assert!(manager.last_notification().is_none());

    // Disable notifications
    manager.set_enabled(false);
    assert!(!manager.is_enabled());

    // When disabled, notify_correction should return false and not send
    let sent_disabled = manager.notify_correction("test", "ทดสอบ");
    assert!(!sent_disabled);
    assert!(manager.last_notification().is_none());
}

#[test]
fn test_notification_manager_formatting_and_unicode() {
    let manager = NotificationManager::new(true);

    // Test Thai to English correction
    manager.notify_correction("้ำสสน", "hello ");
    let (title, body) = manager
        .last_notification()
        .expect("Notification must be recorded");
    assert_eq!(title, "RightLang");
    assert_eq!(body, "้ำสสน → hello");

    // Test trimming whitespace
    manager.notify_correction("   word   ", "   คำ   ");
    let (_, body2) = manager.last_notification().expect("Recorded");
    assert_eq!(body2, "word → คำ");
}

#[test]
fn test_tray_action_from_id_mapping() {
    assert_eq!(
        TrayManager::action_from_id("set_auto"),
        Some(TrayAction::SetMode(CorrectionMode::Auto))
    );
    assert_eq!(
        TrayManager::action_from_id("set_manual"),
        Some(TrayAction::SetMode(CorrectionMode::Manual))
    );
    assert_eq!(
        TrayManager::action_from_id("set_disabled"),
        Some(TrayAction::SetMode(CorrectionMode::Disabled))
    );
    assert_eq!(
        TrayManager::action_from_id("open_config"),
        Some(TrayAction::OpenConfigDir)
    );
    assert_eq!(
        TrayManager::action_from_id("quit"),
        Some(TrayAction::Quit)
    );
    assert_eq!(TrayManager::action_from_id("unknown_id"), None);
    assert_eq!(TrayManager::action_from_id(""), None);
}

#[test]
fn test_tray_manager_initialization_and_mode_switching() {
    let mut tray = TrayManager::new(CorrectionMode::Auto)
        .expect("TrayManager initialization should succeed gracefully");

    assert_eq!(tray.current_mode(), CorrectionMode::Auto);

    tray.set_mode(CorrectionMode::Manual);
    assert_eq!(tray.current_mode(), CorrectionMode::Manual);

    tray.set_mode(CorrectionMode::Disabled);
    assert_eq!(tray.current_mode(), CorrectionMode::Disabled);

    tray.set_mode(CorrectionMode::Auto);
    assert_eq!(tray.current_mode(), CorrectionMode::Auto);
}

#[test]
fn test_tray_dock_policy_execution() {
    // Calling setup_dock_policy must execute safely without panics
    TrayManager::setup_dock_policy();
}

#[test]
fn test_tray_open_config_dir() {
    // open_config_dir ensures ~/.config/rightlang exists
    let res = TrayManager::open_config_dir();
    assert!(res.is_ok());
}
