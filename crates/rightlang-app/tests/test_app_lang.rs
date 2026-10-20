//! Tests for AppLanguageManager in rightlang-app.

use rightlang_app::app_lang::AppLanguageManager;

#[test]
fn test_app_language_manager_dev_apps() {
    let mgr = AppLanguageManager::new();

    // Verify developer / terminal apps detection
    assert!(mgr.is_developer_app("com.apple.Terminal"));
    assert!(mgr.is_developer_app("com.googlecode.iterm2"));
    assert!(mgr.is_developer_app("com.microsoft.VSCode"));
    assert!(mgr.is_developer_app("com.todesktop.230313mzl4w4u92")); // Cursor
    assert!(mgr.is_developer_app("com.apple.dt.Xcode"));
    assert!(mgr.is_developer_app("dev.warp.Warp-Stable"));

    // Normal non-developer apps
    assert!(!mgr.is_developer_app("jp.naver.line.mac"));
    assert!(!mgr.is_developer_app("com.tinyspeck.slackmacgap"));
    assert!(!mgr.is_developer_app("com.google.Chrome"));
}

#[test]
fn test_app_language_manager_memory_set_and_get() {
    let mut mgr = AppLanguageManager::new();

    mgr.set_app_source("jp.naver.line.mac", "com.apple.keylayout.Thai");
    mgr.set_app_source("com.microsoft.VSCode", "com.apple.keylayout.ABC");

    assert_eq!(
        mgr.get_app_source("jp.naver.line.mac"),
        Some("com.apple.keylayout.Thai")
    );
    assert_eq!(
        mgr.get_app_source("com.microsoft.VSCode"),
        Some("com.apple.keylayout.ABC")
    );
    assert_eq!(mgr.get_app_source("com.google.Chrome"), None);
}

#[test]
fn test_app_language_manager_on_app_activated() {
    let mut mgr = AppLanguageManager::new();

    // Simulate switching to LINE
    mgr.on_app_activated("jp.naver.line.mac");
    assert_eq!(mgr.last_active_app(), Some("jp.naver.line.mac"));

    // Simulate switching to VSCode (developer app)
    mgr.on_app_activated("com.microsoft.VSCode");
    assert_eq!(mgr.last_active_app(), Some("com.microsoft.VSCode"));

    // Same app activation should be a no-op
    mgr.on_app_activated("com.microsoft.VSCode");
    assert_eq!(mgr.last_active_app(), Some("com.microsoft.VSCode"));
}
