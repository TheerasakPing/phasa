//! Tests for SmartBypass in rightlang-core.

use rightlang_core::bypass::SmartBypass;

#[test]
fn test_url_and_web_bypass() {
    assert!(SmartBypass::should_bypass("https://google.com"));
    assert!(SmartBypass::should_bypass("http://localhost:8080"));
    assert!(SmartBypass::should_bypass("www.github.com"));
    assert!(SmartBypass::should_bypass("api.openai.com"));
    assert!(SmartBypass::should_bypass("ftp://server.org"));
}

#[test]
fn test_email_bypass() {
    assert!(SmartBypass::should_bypass("admin@company.com"));
    assert!(SmartBypass::should_bypass("support@service.co.th"));
}

#[test]
fn test_file_paths_and_extensions() {
    assert!(SmartBypass::should_bypass("/usr/local/bin"));
    assert!(SmartBypass::should_bypass("./src/main.rs"));
    assert!(SmartBypass::should_bypass("../config.toml"));
    assert!(SmartBypass::should_bypass("~/Documents/file.txt"));
    assert!(SmartBypass::should_bypass("index.html"));
    assert!(SmartBypass::should_bypass("script.py"));
}

#[test]
fn test_cli_flags_and_arguments() {
    assert!(SmartBypass::should_bypass("--version"));
    assert!(SmartBypass::should_bypass("--help"));
    assert!(SmartBypass::should_bypass("-rf"));
    assert!(SmartBypass::should_bypass("-p"));
}

#[test]
fn test_code_identifiers_and_symbols() {
    assert!(SmartBypass::should_bypass("get_user_by_id"));
    assert!(SmartBypass::should_bypass("MAX_BUFFER_SIZE"));
    assert!(SmartBypass::should_bypass("0xdeadbeef"));
    assert!(SmartBypass::should_bypass("v0.1.1"));
    assert!(SmartBypass::should_bypass("std::sync::Arc"));
}

#[test]
fn test_normal_words_do_not_bypass() {
    // Ordinary typed words that should still be checked and converted
    assert!(!SmartBypass::should_bypass("8iy[")); // ครับ
    assert!(!SmartBypass::should_bypass("l;ylfu")); // สวัสดี
    assert!(!SmartBypass::should_bypass("gxHo")); // เป็น (has uppercase H)
    assert!(!SmartBypass::should_bypass("hello"));
    assert!(!SmartBypass::should_bypass("world"));
    assert!(!SmartBypass::should_bypass("ภาษา"));
}
