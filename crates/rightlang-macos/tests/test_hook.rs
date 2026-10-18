use std::sync::Arc;
use std::time::Duration;

use rightlang_macos::event::{
    extract_characters, is_event_tagged_as_rightlang, is_rightlang_event, tag_event_as_rightlang,
    KeyEvent, KeyEventType, RIGHTLANG_EVENT_TAG,
};
use rightlang_macos::ffi::{
    CFRelease, CGEventCreateKeyboardEvent, CGEventKeyboardSetUnicodeString,
};
use rightlang_macos::hook::{is_accessibility_trusted, EventTapListener};

#[test]
fn test_key_event_creation_and_properties() {
    let event = KeyEvent::new(
        0, // macOS KeyCode for 'A'
        KeyEventType::KeyDown,
        0x00020000, // Shift flag mask
        Some("A".to_string()),
        false,
    );

    assert_eq!(event.key_code, 0);
    assert_eq!(event.event_type, KeyEventType::KeyDown);
    assert_eq!(event.flags, 0x00020000);
    assert_eq!(event.characters.as_deref(), Some("A"));
    assert!(!event.is_synthetic);
}

#[test]
fn test_key_event_serialization() {
    let original = KeyEvent::new(
        13, // macOS KeyCode for 'W'
        KeyEventType::KeyUp,
        0,
        Some("w".to_string()),
        true,
    );

    let serialized = serde_json::to_string(&original).expect("Failed to serialize KeyEvent");
    let deserialized: KeyEvent =
        serde_json::from_str(&serialized).expect("Failed to deserialize KeyEvent");

    assert_eq!(original, deserialized);
    assert_eq!(deserialized.key_code, 13);
    assert_eq!(deserialized.event_type, KeyEventType::KeyUp);
    assert!(deserialized.is_synthetic);

    // Test flags changed variant
    let flags_event = KeyEvent::new(56, KeyEventType::FlagsChanged, 0x00020000, None, false);
    let s = serde_json::to_string(&flags_event).expect("Serialize flags event");
    let d: KeyEvent = serde_json::from_str(&s).expect("Deserialize flags event");
    assert_eq!(flags_event, d);
}

#[test]
fn test_rightlang_event_tagging() {
    assert_eq!(RIGHTLANG_EVENT_TAG, 0x524C4730);
    assert!(is_rightlang_event(RIGHTLANG_EVENT_TAG));
    assert!(!is_rightlang_event(0));
    assert!(!is_rightlang_event(12345));

    unsafe {
        // Create a real mock CGEvent
        let cg_event = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0, true);
        assert!(!cg_event.is_null());

        // Initially, the event must not be tagged as RightLang
        assert!(!is_event_tagged_as_rightlang(cg_event));

        // Tag the event
        tag_event_as_rightlang(cg_event);
        assert!(is_event_tagged_as_rightlang(cg_event));

        CFRelease(cg_event as _);
    }
}

#[test]
fn test_channel_message_passing_mock_events() {
    let (tx, rx) = crossbeam_channel::unbounded::<KeyEvent>();

    let events = vec![
        KeyEvent::new(0, KeyEventType::KeyDown, 0, Some("a".into()), false),
        KeyEvent::new(0, KeyEventType::KeyUp, 0, Some("a".into()), false),
        KeyEvent::new(1, KeyEventType::KeyDown, 0, Some("s".into()), false),
        KeyEvent::new(56, KeyEventType::FlagsChanged, 0x00020000, None, false),
    ];

    for ev in &events {
        tx.send(ev.clone()).expect("Failed to send KeyEvent");
    }

    for expected in events {
        let received = rx
            .recv_timeout(Duration::from_millis(100))
            .expect("Failed to receive KeyEvent");
        assert_eq!(received, expected);
    }
}

#[test]
fn test_extract_characters_from_cg_event() {
    unsafe {
        let cg_event = CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0x0B, true); // 0x0B = 'B'
        assert!(!cg_event.is_null());

        let test_chars: Vec<u16> = "hello".encode_utf16().collect();
        CGEventKeyboardSetUnicodeString(
            cg_event,
            test_chars.len() as libc::c_ulong,
            test_chars.as_ptr(),
        );

        let extracted = extract_characters(cg_event);
        assert_eq!(extracted, Some("hello".to_string()));

        CFRelease(cg_event as _);
    }
}

#[test]
fn test_accessibility_check_api() {
    // Check without prompt - should return a boolean without panicking
    let trusted = is_accessibility_trusted(false);
    println!("Accessibility trusted status: {}", trusted);
}

#[test]
fn test_event_tap_lifecycle_and_stop() {
    let trusted = is_accessibility_trusted(false);
    if !trusted {
        eprintln!("Accessibility not trusted in test environment; skipping live tap test");
        return;
    }

    let (tx, _rx) = crossbeam_channel::unbounded();
    let listener = Arc::new(EventTapListener::new(tx).expect("Failed to create EventTapListener"));
    assert!(!listener.is_running());

    let listener_clone = Arc::clone(&listener);
    let worker_thread = std::thread::spawn(move || listener_clone.run());

    // Wait briefly for runloop to start
    std::thread::sleep(Duration::from_millis(60));
    assert!(listener.is_running());

    // Stop listener
    listener.stop();

    // Verify worker thread terminates cleanly
    let result = worker_thread.join().expect("Worker thread panicked");
    assert!(result.is_ok());
    assert!(!listener.is_running());
}
