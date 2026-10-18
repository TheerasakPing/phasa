use rightlang_macos::clipboard::{get_clipboard_text, set_clipboard_text};
use rightlang_macos::dispatcher::KeyDispatcher;
use rightlang_macos::ffi::{KEY_CODE_C, K_CG_EVENT_FLAG_MASK_COMMAND};

#[test]
fn test_clipboard_set_and_get_ascii() {
    let test_str = "RightLang Clipboard Test 12345";
    let success = set_clipboard_text(test_str);
    assert!(success, "Setting clipboard text should succeed");

    let read_back = get_clipboard_text().expect("Clipboard should contain text");
    assert_eq!(read_back, test_str);
}

#[test]
fn test_clipboard_set_and_get_thai_unicode() {
    let thai_str = "สวัสดีชาวโลก ทดสอบคลิปบอร์ด ๕๕๕";
    let success = set_clipboard_text(thai_str);
    assert!(success, "Setting Thai clipboard text should succeed");

    let read_back = get_clipboard_text().expect("Clipboard should contain Thai text");
    assert_eq!(read_back, thai_str);
}

#[test]
fn test_clipboard_multiline_and_symbols() {
    let complex_str = "Line 1: !@#$%^&*()\nLine 2: ภาษาไทย + English\nLine 3: 🦀🦀🦀";
    let success = set_clipboard_text(complex_str);
    assert!(success, "Setting multiline text should succeed");

    let read_back = get_clipboard_text().expect("Clipboard should contain multiline text");
    assert_eq!(read_back, complex_str);
}

#[test]
fn test_send_key_combination_execution() {
    let dispatcher = KeyDispatcher::new(500);
    // Dispatches synthetic Cmd+C with microsecond delay without crashing
    let res = dispatcher.send_key_combination(KEY_CODE_C, K_CG_EVENT_FLAG_MASK_COMMAND, 500);
    assert!(res.is_ok(), "send_key_combination should execute without error");
}
