use core_foundation::base::{CFRelease, CFTypeRef};
use rightlang_macos::dispatcher::KeyDispatcher;
use rightlang_macos::event::{extract_characters, is_event_tagged_as_rightlang};
use rightlang_macos::ffi::{
    CGEventGetIntegerValueField, KEY_CODE_BACKSPACE, K_CG_KEYBOARD_EVENT_KEYCODE,
};
use rightlang_macos::tis::{InputSourceManager, TisError};

#[test]
fn test_query_current_input_source() {
    let manager = InputSourceManager::new();
    let current_id = manager.get_current_source_id();
    assert!(
        current_id.is_ok(),
        "Expected current input source ID, got error: {:?}",
        current_id.err()
    );
    let id_str = current_id.unwrap();
    println!("Current active input source ID: {}", id_str);
    assert!(!id_str.is_empty());
}

#[test]
fn test_list_keyboard_input_sources() {
    let manager = InputSourceManager::new();
    let sources = manager
        .list_keyboard_input_sources()
        .expect("Failed to list enabled keyboard input sources");
    println!("Enabled keyboard sources count: {}", sources.len());
    assert!(
        !sources.is_empty(),
        "At least one keyboard input source must be enabled on macOS"
    );

    for s in &sources {
        println!("  - ID: {}, Name: {}, Languages: {:?}", s.id, s.name, s.languages);
        assert!(!s.id.is_empty());
        assert!(!s.name.is_empty());
    }

    let all_sources = manager
        .list_all_installed_keyboard_input_sources()
        .expect("Failed to list installed keyboard input sources");
    assert!(all_sources.len() >= sources.len());
}

#[test]
fn test_select_input_source_not_found() {
    let manager = InputSourceManager::new();
    let res = manager.select_input_source("com.apple.nonexistent.keyboard.layout.12345");
    assert!(res.is_err());
    match res {
        Err(TisError::SourceNotFound(id)) => {
            assert_eq!(id, "com.apple.nonexistent.keyboard.layout.12345");
        }
        other => panic!("Expected SourceNotFound error, got {:?}", other),
    }
}

#[test]
fn test_switch_input_sources_and_restore() {
    let manager = InputSourceManager::new();
    let initial_id = manager
        .get_current_source_id()
        .expect("Failed to get current input source ID");
    println!("Initial source ID: {}", initial_id);

    // Test switching to Thai (if available on system)
    let thai_available = manager.switch_to_thai().expect("switch_to_thai failed");
    println!("Thai available: {}", thai_available);
    if thai_available {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let current_after_thai = manager
            .get_current_source_id()
            .expect("Failed to get current source after switching to Thai");
        println!("Current after thai: {}", current_after_thai);
        assert!(
            current_after_thai.to_lowercase().contains("thai"),
            "Expected Thai layout active, got: {}",
            current_after_thai
        );
    }

    // Test switching to English
    let english_available = manager
        .switch_to_english()
        .expect("switch_to_english failed");
    println!("English available: {}", english_available);
    if english_available {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let current_after_en = manager
            .get_current_source_id()
            .expect("Failed to get current source after switching to English");
        println!("Current after en: {}", current_after_en);
        assert!(
            current_after_en.contains("ABC")
                || current_after_en.contains("US")
                || current_after_en.to_lowercase().contains("en"),
            "Expected English layout active, got: {}",
            current_after_en
        );
    }

    // Restore initial input source
    manager
        .select_input_source(&initial_id)
        .expect("Failed to restore initial input source");
    std::thread::sleep(std::time::Duration::from_millis(50));
    let restored_id = manager
        .get_current_source_id()
        .expect("Failed to get restored source ID");
    assert_eq!(initial_id, restored_id);
}

#[test]
fn test_tis_thread_safety_from_background_thread() {
    let handle = std::thread::spawn(|| {
        let manager = InputSourceManager::new();
        let current = manager.get_current_source_id();
        assert!(current.is_ok());
        let sources = manager.list_keyboard_input_sources();
        assert!(sources.is_ok());
    });
    handle.join().expect("Background thread panicked");
}

#[test]
fn test_key_dispatcher_construction() {
    let dispatcher = KeyDispatcher::new(500);
    assert_eq!(dispatcher.default_delay_us(), 500);

    let default_dispatcher = KeyDispatcher::default();
    assert_eq!(default_dispatcher.default_delay_us(), 2000);
}

#[test]
fn test_key_dispatcher_backspace_event_generation() {
    unsafe {
        // Key down
        let event_down = KeyDispatcher::create_backspace_event(true)
            .expect("Failed to create backspace KeyDown event");
        assert!(!event_down.is_null());
        assert!(
            is_event_tagged_as_rightlang(event_down),
            "Backspace event must be tagged with RIGHTLANG_EVENT_TAG"
        );
        let key_code = CGEventGetIntegerValueField(event_down, K_CG_KEYBOARD_EVENT_KEYCODE);
        assert_eq!(
            key_code as u16, KEY_CODE_BACKSPACE,
            "Backspace key code must be 0x33"
        );
        CFRelease(event_down as CFTypeRef);

        // Key up
        let event_up = KeyDispatcher::create_backspace_event(false)
            .expect("Failed to create backspace KeyUp event");
        assert!(!event_up.is_null());
        assert!(
            is_event_tagged_as_rightlang(event_up),
            "Backspace KeyUp event must be tagged with RIGHTLANG_EVENT_TAG"
        );
        CFRelease(event_up as CFTypeRef);
    }
}

#[test]
fn test_key_dispatcher_unicode_event_generation() {
    unsafe {
        // 1. ASCII character 'a'
        let mut buf_a = [0u16; 2];
        let utf16_a = 'a'.encode_utf16(&mut buf_a);
        let event_a = KeyDispatcher::create_unicode_event(utf16_a, true)
            .expect("Failed to create unicode event for 'a'");
        assert!(is_event_tagged_as_rightlang(event_a));
        assert_eq!(extract_characters(event_a), Some("a".to_string()));
        CFRelease(event_a as CFTypeRef);

        // 2. Thai character 'ก' (U+0E01)
        let mut buf_th = [0u16; 2];
        let utf16_th = 'ก'.encode_utf16(&mut buf_th);
        let event_th = KeyDispatcher::create_unicode_event(utf16_th, true)
            .expect("Failed to create unicode event for 'ก'");
        assert!(is_event_tagged_as_rightlang(event_th));
        assert_eq!(extract_characters(event_th), Some("ก".to_string()));
        CFRelease(event_th as CFTypeRef);

        // 3. Thai combining tone mark '้' (U+0E49)
        let mut buf_tone = [0u16; 2];
        let utf16_tone = '้'.encode_utf16(&mut buf_tone);
        let event_tone = KeyDispatcher::create_unicode_event(utf16_tone, true)
            .expect("Failed to create unicode event for '้'");
        assert!(is_event_tagged_as_rightlang(event_tone));
        assert_eq!(extract_characters(event_tone), Some("้".to_string()));
        CFRelease(event_tone as CFTypeRef);

        // 4. Astral plane surrogate pair emoji '🦀' (U+1F980)
        let mut buf_crab = [0u16; 2];
        let utf16_crab = '🦀'.encode_utf16(&mut buf_crab);
        assert_eq!(
            utf16_crab.len(),
            2,
            "Crab emoji must require 2 UTF-16 surrogate units"
        );
        let event_crab = KeyDispatcher::create_unicode_event(utf16_crab, true)
            .expect("Failed to create unicode event for '🦀'");
        assert!(is_event_tagged_as_rightlang(event_crab));
        assert_eq!(extract_characters(event_crab), Some("🦀".to_string()));
        CFRelease(event_crab as CFTypeRef);
    }
}

#[test]
fn test_key_dispatcher_edge_cases() {
    let dispatcher = KeyDispatcher::new(100);
    // Zero count backspaces should be an immediate no-op Ok(())
    assert!(dispatcher.send_backspaces(0, 0).is_ok());
    // Empty text should be an immediate no-op Ok(())
    assert!(dispatcher.send_text("", 0).is_ok());
    // Replace empty with zero backspaces
    assert!(dispatcher.replace_text(0, "", None).is_ok());
}
