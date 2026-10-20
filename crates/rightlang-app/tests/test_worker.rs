use rightlang_app::config::{AppConfig, CorrectionMode};
use rightlang_app::worker::KeyWorker;
use rightlang_core::DictionaryFilter;
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
    let mut dict = DictionaryFilter::new();
    dict.load_thai_str(include_str!("../../../assets/thai.txt"));
    dict.load_eng_str(include_str!("../../../assets/eng.txt"));
    dict.shrink_to_fit();

    let mut worker = KeyWorker::new(config, dict);
    if let Ok(td) = tempfile::tempdir() {
        let path = td.path().to_path_buf();
        std::mem::forget(td);
        worker.set_user_dict_paths(path.join("user.th.txt"), path.join("user.en.txt"));
    }
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

    // Followed by space -> triggers correction to "ครับ " in Thai with trailing space!
    let record = worker
        .handle_event(&create_mock_key_event(" "))
        .expect("Space must trigger correction for '8iy['");

    assert_eq!(record.original, "8iy[");
    assert_eq!(record.replacement, "ครับ ");
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

    // Followed by space -> triggers correction to "เป็น " in Thai with trailing space!
    let record = worker
        .handle_event(&create_mock_key_event(" "))
        .expect("Space must trigger correction for 'gxHo'");

    assert_eq!(record.original, "gxHo");
    assert_eq!(record.replacement, "เป็น ");
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

    worker.handle_event(&create_mock_backspace_event());
    assert_eq!(worker.buffer(), "ab");

    worker.handle_event(&create_mock_backspace_event());
    assert_eq!(worker.buffer(), "a");

    worker.handle_event(&create_mock_backspace_event());
    assert_eq!(worker.buffer(), "");

    // Popping empty buffer should not panic
    worker.handle_event(&create_mock_backspace_event());
    assert_eq!(worker.buffer(), "");
}

#[test]
fn test_worker_disabled_application_rule() {
    let mut worker = create_test_worker();

    // Set active app to a disabled bundle ID (e.g. Terminal)
    worker.set_active_app(Some("com.apple.Terminal".to_string()));
    worker
        .rule_manager_mut()
        .set_rule("com.apple.Terminal", CorrectionMode::Disabled);

    for ch in ["8", "i", "y", "[", " "] {
        assert!(worker.handle_event(&create_mock_key_event(ch)).is_none());
    }

    assert_eq!(worker.buffer(), "");
    assert!(worker.last_correction().is_none());
}

#[test]
fn test_worker_undo_restores_previous_buffer() {
    let mut worker = create_test_worker();

    for ch in ["8", "i", "y", "[", " "] {
        worker.handle_event(&create_mock_key_event(ch));
    }

    let record = worker
        .last_correction()
        .expect("Correction expected");
    assert_eq!(record.replacement, "ครับ ");
    assert_eq!(worker.buffer(), "");

    // Test Undo: restores previous buffer
    assert!(worker.undo(), "First undo must succeed");
    assert_eq!(worker.buffer(), "8iy[", "Buffer must be restored to previous state");
    assert!(worker.last_correction().is_none());

    // Subsequent undo fails
    assert!(!worker.undo(), "Subsequent undo must return false");
}

#[test]
fn test_worker_undo_shortcuts() {
    let mut worker = create_test_worker();

    // 1. Trigger correction
    for ch in ["8", "i", "y", "[", " "] {
        worker.handle_event(&create_mock_key_event(ch));
    }
    assert!(worker.last_correction().is_some());

    // Cmd+Shift+Z shortcut (Cmd = 0x00100000, Shift = 0x00020000, keycode = 0x06)
    const CMD_FLAG: u64 = 0x00100000;
    const SHIFT_FLAG: u64 = 0x00020000;
    let cmd_shift_z = KeyEvent::new(0x06, KeyEventType::KeyDown, CMD_FLAG | SHIFT_FLAG, Some("Z".into()), false);
    assert!(worker.handle_event(&cmd_shift_z).is_none());
    assert_eq!(worker.buffer(), "8iy[");
    assert!(worker.last_correction().is_none());

    // 2. Trigger correction again
    worker.clear_buffer();
    for ch in ["g", "x", "H", "o", " "] {
        worker.handle_event(&create_mock_key_event(ch));
    }
    assert!(worker.last_correction().is_some());

    // Ctrl+Option+Z shortcut (Ctrl = 0x00040000, Option = 0x00080000, keycode = 0x06)
    const CTRL_FLAG: u64 = 0x00040000;
    const ALT_FLAG: u64 = 0x00080000;
    let ctrl_opt_z = KeyEvent::new(0x06, KeyEventType::KeyDown, CTRL_FLAG | ALT_FLAG, Some("z".into()), false);
    assert!(worker.handle_event(&ctrl_opt_z).is_none());
    assert_eq!(worker.buffer(), "gxHo");
    assert!(worker.last_correction().is_none());
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
    assert_eq!(record.replacement, "hello ");
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

    // Command key modifier (Cmd+K)
    worker.handle_event(&create_mock_key_event("c"));
    assert_eq!(worker.buffer(), "c");
    let cmd_k_event = KeyEvent::new(0x28, KeyEventType::KeyDown, 0x00100000, Some("k".into()), false);
    assert!(worker.handle_event(&cmd_k_event).is_none());
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

    // In Manual mode, regular space does NOT trigger automatic correction
    let result = worker.handle_event(&create_mock_key_event(" "));
    assert!(result.is_none());

    // Explicit manual evaluate_and_correct() still works and does NOT append space
    worker.handle_event(&create_mock_key_event("8"));
    worker.handle_event(&create_mock_key_event("i"));
    worker.handle_event(&create_mock_key_event("y"));
    worker.handle_event(&create_mock_key_event("["));
    let manual_record = worker.evaluate_and_correct().expect("Manual evaluate should work");
    assert_eq!(manual_record.replacement, "ครับ");
}

#[test]
fn test_worker_manual_mode_option_space_hotkey() {
    let mut worker = create_test_worker();
    worker.set_mode(CorrectionMode::Manual);

    // Type "8iy["
    worker.handle_event(&create_mock_key_event("8"));
    worker.handle_event(&create_mock_key_event("i"));
    worker.handle_event(&create_mock_key_event("y"));
    worker.handle_event(&create_mock_key_event("["));
    assert_eq!(worker.buffer(), "8iy[");

    // Option + Space hotkey (keycode 0x31, ALT_FLAG 0x00080000)
    const ALT_FLAG: u64 = 0x00080000;
    let opt_space_event = KeyEvent::new(0x31, KeyEventType::KeyDown, ALT_FLAG, Some(" ".into()), false);

    let record = worker
        .handle_event(&opt_space_event)
        .expect("Option+Space must trigger manual correction");

    assert_eq!(record.original, "8iy[");
    assert_eq!(record.replacement, "ครับ");
    assert_eq!(record.target_language, Language::Thai);
    assert_eq!(worker.buffer(), "");

    // Also verify with device alternate flag 0x00000020
    worker.handle_event(&create_mock_key_event("g"));
    worker.handle_event(&create_mock_key_event("x"));
    worker.handle_event(&create_mock_key_event("H"));
    worker.handle_event(&create_mock_key_event("o"));
    let opt_dev_space_event = KeyEvent::new(0x31, KeyEventType::KeyDown, 0x00000020, Some(" ".into()), false);
    let record2 = worker
        .handle_event(&opt_dev_space_event)
        .expect("Option+Space with device flag must trigger manual correction");
    assert_eq!(record2.replacement, "เป็น");

    // In Auto mode, Option+Space does NOT trigger manual correction
    worker.set_mode(CorrectionMode::Auto);
    worker.handle_event(&create_mock_key_event("8"));
    worker.handle_event(&create_mock_key_event("i"));
    let auto_opt_space = worker.handle_event(&opt_space_event);
    assert!(auto_opt_space.is_none());
    assert_eq!(worker.buffer(), "");
}

#[test]
fn test_worker_convert_selection_and_clipboard() {
    let mut worker = create_test_worker();

    // 1. English -> Thai via convert_clipboard_content
    let record_th = worker
        .convert_clipboard_content("8iy[")
        .expect("Should convert '8iy[' to 'ครับ'");
    assert_eq!(record_th.original, "8iy[");
    assert_eq!(record_th.replacement, "ครับ");
    assert_eq!(record_th.target_language, Language::Thai);

    // Verify clipboard was updated with translated text
    let clip_text = rightlang_macos::clipboard::get_clipboard_text().expect("Clipboard has text");
    assert_eq!(clip_text, "ครับ");

    // 2. Thai -> English via convert_clipboard_content
    let record_en = worker
        .convert_clipboard_content("้ำสสน")
        .expect("Should convert '้ำสสน' to 'hello'");
    assert_eq!(record_en.original, "้ำสสน");
    assert_eq!(record_en.replacement, "hello");
    assert_eq!(record_en.target_language, Language::English);

    // 3. Empty text -> returns None
    assert!(worker.convert_clipboard_content("   ").is_none());

    // 4. Cmd + / shortcut (keycode 0x2C, CMD_FLAG = 0x00100000)
    rightlang_macos::clipboard::set_clipboard_text("8iy[");
    const CMD_FLAG: u64 = 0x00100000;
    let cmd_slash_event =
        KeyEvent::new(0x2C, KeyEventType::KeyDown, CMD_FLAG, Some("/".into()), false);

    let record = worker.handle_event(&cmd_slash_event);
    assert!(
        record.is_some(),
        "Cmd + / should trigger selection conversion"
    );
    let rec = record.unwrap();
    assert_eq!(rec.replacement, "ครับ");

    // 5. Ctrl + Option + C shortcut (keycode 0x08, CTRL_FLAG | ALT_FLAG)
    rightlang_macos::clipboard::set_clipboard_text("้ำสสน");
    const CTRL_FLAG: u64 = 0x00040000;
    const ALT_FLAG: u64 = 0x00080000;
    let ctrl_opt_c_event = KeyEvent::new(
        0x08,
        KeyEventType::KeyDown,
        CTRL_FLAG | ALT_FLAG,
        Some("c".into()),
        false,
    );

    let record2 = worker.handle_event(&ctrl_opt_c_event);
    assert!(
        record2.is_some(),
        "Ctrl + Option + C should trigger selection conversion"
    );
    let rec2 = record2.unwrap();
    assert_eq!(rec2.replacement, "hello");
}

#[test]
fn test_worker_auto_learn_and_undo_retention() {
    let mut worker = create_test_worker();
    worker.config_mut().auto_learn = true;

    let tmp_dir = tempfile::tempdir().expect("create tempdir");
    let thai_file = tmp_dir.path().join("user.th.txt");
    let eng_file = tmp_dir.path().join("user.en.txt");
    worker.set_user_dict_paths(&thai_file, &eng_file);

    // 1. Correcting "8iy[" -> "ครับ" should auto-learn "ครับ" and save to disk
    for ch in "8iy[".chars() {
        worker.handle_event(&create_mock_key_event(&ch.to_string()));
    }
    let record = worker.handle_event(&create_mock_key_event(" ")).expect("correction");
    assert_eq!(record.replacement, "ครับ ");

    assert!(worker.dictionary().user_thai_words().contains("ครับ"));
    assert!(thai_file.exists());

    // 2. Undoing the correction should learn original "8iy[" as an English user word
    assert!(worker.undo());
    assert!(worker.dictionary().user_eng_words().contains("8iy["));
    assert!(eng_file.exists());

    // 3. Since "8iy[" is now known in user_eng_words, typing it again will NOT trigger correction!
    worker.clear_buffer();
    for ch in "8iy[".chars() {
        worker.handle_event(&create_mock_key_event(&ch.to_string()));
    }
    let second_res = worker.handle_event(&create_mock_key_event(" "));
    assert!(second_res.is_none(), "Learned word must not be re-corrected after undo");

    // 4. Test explicit learn_word API
    assert!(worker.learn_word("customterm", Language::English));
    assert!(worker.dictionary().contains_english("customterm"));
}

#[test]
fn test_worker_idle_buffer_flush() {
    let mut worker = create_test_worker();
    worker.set_buffer_idle_timeout(std::time::Duration::from_millis(50));

    // Type "8iy["
    for ch in "8iy[".chars() {
        worker.handle_event(&create_mock_key_event(&ch.to_string()));
    }
    assert_eq!(worker.buffer(), "8iy[");

    // Sleep for 60ms to exceed idle timeout
    std::thread::sleep(std::time::Duration::from_millis(60));

    // Typing a new character after being idle should flush the stale buffer
    worker.handle_event(&create_mock_key_event("a"));
    assert_eq!(worker.buffer(), "a");
}
