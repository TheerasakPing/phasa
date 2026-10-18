use rightlang_app::config::{AppConfig, CorrectionMode};
use rightlang_app::worker::{load_dictionaries, KeyWorker};
use rightlang_core::Language;
use rightlang_macos::event::{KeyEvent, KeyEventType};

fn create_mock_key_event(ch: &str) -> KeyEvent {
    let key_code = match ch {
        " " => 0x31,
        _ => 0,
    };
    KeyEvent::new(
        key_code,
        KeyEventType::KeyDown,
        0,
        Some(ch.to_string()),
        false,
    )
}

fn create_mock_backspace_event() -> KeyEvent {
    KeyEvent::new(0x33, KeyEventType::KeyDown, 0, None, false)
}

fn create_test_worker() -> KeyWorker {
    let config = AppConfig {
        key_delay_us: 0,
        ..Default::default()
    };
    let dict = load_dictionaries();
    let mut worker = KeyWorker::new(config, dict);
    worker.set_active_app(Some("com.apple.Safari".to_string()));
    worker
}

#[test]
fn test_worker_correction_khrap() {
    let mut worker = create_test_worker();

    // Send KeyEvents for "8", "i", "y", "["
    assert!(worker.handle_event(&create_mock_key_event("8")).is_none());
    assert_eq!(worker.buffer(), "8");

    assert!(worker.handle_event(&create_mock_key_event("i")).is_none());
    assert_eq!(worker.buffer(), "8i");

    assert!(worker.handle_event(&create_mock_key_event("y")).is_none());
    assert_eq!(worker.buffer(), "8iy");

    assert!(worker.handle_event(&create_mock_key_event("[")).is_none());
    assert_eq!(worker.buffer(), "8iy[");

    // Followed by space -> triggers correction to "ครับ" in Thai!
    let record = worker
        .handle_event(&create_mock_key_event(" "))
        .expect("Space must trigger correction for '8iy['");

    assert_eq!(record.original, "8iy[");
    assert_eq!(record.replacement, "ครับ");
    assert_eq!(record.target_language, Language::Thai);
    assert_eq!(worker.buffer(), "");
}

#[test]
fn test_worker_correction_pen() {
    let mut worker = create_test_worker();

    // Send KeyEvents for "g", "x", "H", "o"
    assert!(worker.handle_event(&create_mock_key_event("g")).is_none());
    assert!(worker.handle_event(&create_mock_key_event("x")).is_none());
    assert!(worker.handle_event(&create_mock_key_event("H")).is_none());
    assert!(worker.handle_event(&create_mock_key_event("o")).is_none());
    assert_eq!(worker.buffer(), "gxHo");

    // Followed by space -> triggers correction to "เป็น" in Thai!
    let record = worker
        .handle_event(&create_mock_key_event(" "))
        .expect("Space must trigger correction for 'gxHo'");

    assert_eq!(record.original, "gxHo");
    assert_eq!(record.replacement, "เป็น");
    assert_eq!(record.target_language, Language::Thai);
    assert_eq!(worker.buffer(), "");
}

#[test]
fn test_worker_backspace_pops_last_char() {
    let mut worker = create_test_worker();

    worker.handle_event(&create_mock_key_event("a"));
    worker.handle_event(&create_mock_key_event("b"));
    worker.handle_event(&create_mock_key_event("c"));
    assert_eq!(worker.buffer(), "abc");

    // Send backspace -> pops last char from buffer
    assert!(worker.handle_event(&create_mock_backspace_event()).is_none());
    assert_eq!(worker.buffer(), "ab");

    assert!(worker.handle_event(&create_mock_backspace_event()).is_none());
    assert_eq!(worker.buffer(), "a");

    assert!(worker.handle_event(&create_mock_backspace_event()).is_none());
    assert_eq!(worker.buffer(), "");

    // Popping on empty buffer does not panic or error
    assert!(worker.handle_event(&create_mock_backspace_event()).is_none());
    assert_eq!(worker.buffer(), "");
}

#[test]
fn test_worker_disabled_application_rule() {
    let mut worker = create_test_worker();

    // Set active app to com.apple.Terminal (disabled in default configuration)
    worker.set_active_app(Some("com.apple.Terminal".to_string()));

    worker.handle_event(&create_mock_key_event("8"));
    worker.handle_event(&create_mock_key_event("i"));
    worker.handle_event(&create_mock_key_event("y"));
    worker.handle_event(&create_mock_key_event("["));
    assert_eq!(worker.buffer(), "8iy[");

    // In a disabled application, no correction is triggered
    let result = worker.handle_event(&create_mock_key_event(" "));
    assert!(result.is_none(), "Disabled app must not trigger correction");
    assert_eq!(worker.buffer(), "");
}

#[test]
fn test_worker_undo_restores_previous_buffer() {
    let mut worker = create_test_worker();

    // Send "8", "i", "y", "[", followed by space
    worker.handle_event(&create_mock_key_event("8"));
    worker.handle_event(&create_mock_key_event("i"));
    worker.handle_event(&create_mock_key_event("y"));
    worker.handle_event(&create_mock_key_event("["));

    let record = worker
        .handle_event(&create_mock_key_event(" "))
        .expect("Correction expected");
    assert_eq!(record.replacement, "ครับ");
    assert_eq!(worker.buffer(), "");

    // Test Undo: restores previous buffer
    assert!(worker.undo(), "First undo must succeed");
    assert_eq!(worker.buffer(), "8iy[", "Buffer must be restored to previous state");
    assert!(worker.last_correction().is_none());

    // Subsequent undo fails
    assert!(!worker.undo(), "Subsequent undo must return false");
}

#[test]
fn test_worker_reverse_correction_thai_to_english() {
    let mut worker = create_test_worker();

    // Type Thai characters "้ำสสน" (which is "hello" typed on Thai layout)
    for ch in "้ำสสน".chars() {
        assert!(worker
            .handle_event(&create_mock_key_event(&ch.to_string()))
            .is_none());
    }
    assert_eq!(worker.buffer(), "้ำสสน");

    let record = worker
        .handle_event(&create_mock_key_event(" "))
        .expect("Reverse correction should trigger for '้ำสสน'");

    assert_eq!(record.original, "้ำสสน");
    assert_eq!(record.replacement, "hello");
    assert_eq!(record.target_language, Language::English);
    assert_eq!(worker.buffer(), "");

    // Undo should restore "้ำสสน"
    assert!(worker.undo());
    assert_eq!(worker.buffer(), "้ำสสน");
}

#[test]
fn test_worker_valid_words_not_corrected() {
    let mut worker = create_test_worker();

    // Typing a valid English word "hello" followed by space
    for ch in "hello".chars() {
        worker.handle_event(&create_mock_key_event(&ch.to_string()));
    }
    assert_eq!(worker.buffer(), "hello");

    let result = worker.handle_event(&create_mock_key_event(" "));
    assert!(result.is_none(), "Valid English word should not be corrected");
    assert_eq!(worker.buffer(), "");

    // Typing a valid Thai word "สวัสดี" followed by space
    for ch in "สวัสดี".chars() {
        worker.handle_event(&create_mock_key_event(&ch.to_string()));
    }
    assert_eq!(worker.buffer(), "สวัสดี");

    let result_thai = worker.handle_event(&create_mock_key_event(" "));
    assert!(result_thai.is_none(), "Valid Thai word should not be corrected");
    assert_eq!(worker.buffer(), "");
}

#[test]
fn test_worker_navigation_and_control_keys_clear_buffer() {
    let mut worker = create_test_worker();

    // Enter key (0x24)
    worker.handle_event(&create_mock_key_event("a"));
    assert_eq!(worker.buffer(), "a");
    let enter_event = KeyEvent::new(0x24, KeyEventType::KeyDown, 0, Some("\n".into()), false);
    assert!(worker.handle_event(&enter_event).is_none());
    assert_eq!(worker.buffer(), "");

    // Left Arrow key (0x7B)
    worker.handle_event(&create_mock_key_event("b"));
    assert_eq!(worker.buffer(), "b");
    let arrow_event = KeyEvent::new(0x7B, KeyEventType::KeyDown, 0, None, false);
    assert!(worker.handle_event(&arrow_event).is_none());
    assert_eq!(worker.buffer(), "");

    // Command key modifier (Cmd+C)
    worker.handle_event(&create_mock_key_event("c"));
    assert_eq!(worker.buffer(), "c");
    let cmd_c_event = KeyEvent::new(0x08, KeyEventType::KeyDown, 0x00100000, Some("c".into()), false);
    assert!(worker.handle_event(&cmd_c_event).is_none());
    assert_eq!(worker.buffer(), "");
}

#[test]
fn test_worker_manual_mode() {
    let mut worker = create_test_worker();
    worker.set_mode(CorrectionMode::Manual);

    worker.handle_event(&create_mock_key_event("8"));
    worker.handle_event(&create_mock_key_event("i"));
    worker.handle_event(&create_mock_key_event("y"));
    worker.handle_event(&create_mock_key_event("["));
    assert_eq!(worker.buffer(), "8iy[");

    // In Manual mode, space does NOT trigger automatic correction
    let result = worker.handle_event(&create_mock_key_event(" "));
    assert!(result.is_none());

    // Explicit manual evaluate_and_correct() still works
    worker.handle_event(&create_mock_key_event("8"));
    worker.handle_event(&create_mock_key_event("i"));
    worker.handle_event(&create_mock_key_event("y"));
    worker.handle_event(&create_mock_key_event("["));
    let manual_record = worker.evaluate_and_correct().expect("Manual evaluate should work");
    assert_eq!(manual_record.replacement, "ครับ");
}
