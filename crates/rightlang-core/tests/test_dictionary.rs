use rightlang_core::{
    dictionary::{DictionaryFilter, Language},
    layout::{KeyTranslator, KeyboardLayout},
    trie::CompactTrie,
};
use std::path::Path;
use std::time::Instant;

#[test]
fn test_trie_insert_and_lookup() {
    let mut trie = CompactTrie::new();
    assert!(trie.is_empty());
    assert_eq!(trie.len(), 0);
    assert!(!trie.contains(""));
    assert!(!trie.is_prefix(""));

    // Insert single English word
    trie.insert("apple");
    assert!(!trie.is_empty());
    assert_eq!(trie.len(), 1);
    assert!(trie.contains("apple"));
    assert!(!trie.contains("app"));
    assert!(trie.is_prefix("a"));
    assert!(trie.is_prefix("app"));
    assert!(trie.is_prefix("apple"));
    assert!(!trie.is_prefix("apples"));
    assert!(!trie.is_prefix("b"));

    // Insert prefix word
    trie.insert("app");
    assert_eq!(trie.len(), 2);
    assert!(trie.contains("app"));
    assert!(trie.contains("apple"));

    // Duplicate insert does not change length
    trie.insert("apple");
    assert_eq!(trie.len(), 2);

    // Thai words insertion
    trie.insert("สวัสดี");
    trie.insert("เป็น");
    trie.insert("ครับ");
    assert_eq!(trie.len(), 5);

    assert!(trie.contains("สวัสดี"));
    assert!(trie.contains("เป็น"));
    assert!(trie.contains("ครับ"));

    assert!(trie.is_prefix("ส"));
    assert!(trie.is_prefix("สวัส"));
    assert!(trie.is_prefix("เป็"));
    assert!(trie.is_prefix("ครั"));

    assert!(!trie.contains("สวัส"));
    assert!(!trie.contains("เป็"));
    assert!(!trie.contains("ครั"));
    assert!(!trie.contains("ไม่ใช่"));
}

#[test]
fn test_dictionary_load_assets() {
    let thai_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/thai.txt"
    ));
    let eng_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/eng.txt"));

    let dict = DictionaryFilter::load_from_files(thai_path, eng_path)
        .expect("Failed to load dictionary asset files");

    // Verify word counts
    assert!(
        dict.thai_trie().len() > 20000,
        "Thai trie should contain > 20,000 words"
    );
    assert!(
        dict.eng_trie().len() > 14000,
        "English trie should contain > 14,000 words"
    );

    // Verify key Thai words
    assert!(dict.contains_thai("สวัสดี"));
    assert!(dict.contains_thai("เป็น"));
    assert!(dict.contains_thai("ครับ"));

    // Verify key English words (case insensitive lookup)
    assert!(dict.contains_english("hello"));
    assert!(dict.contains_english("world"));
    assert!(dict.contains_english("Hello"));
    assert!(dict.contains_english("WORLD"));

    // Verify prefixes
    assert!(dict.is_thai_prefix("สวัส"));
    assert!(dict.is_thai_prefix("เป็"));
    assert!(dict.is_thai_prefix("ครั"));
    assert!(dict.is_eng_prefix("hel"));
    assert!(dict.is_eng_prefix("wor"));
    assert!(dict.is_eng_prefix("HeL"));

    // Non-existent words
    assert!(!dict.contains_thai("foobar"));
    assert!(!dict.contains_english("ไม่มีทาง"));
    assert!(!dict.is_eng_prefix("zzzzzz"));
}

#[test]
fn test_detect_language() {
    let thai_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/thai.txt"
    ));
    let eng_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/eng.txt"));

    let dict = DictionaryFilter::load_from_files(thai_path, eng_path)
        .expect("Failed to load dictionary asset files");

    assert_eq!(dict.detect_language("สวัสดี"), Some(Language::Thai));
    assert_eq!(dict.detect_language("hello"), Some(Language::English));
    assert_eq!(dict.detect_language("me"), Some(Language::English));
    assert_eq!(dict.detect_language("if"), Some(Language::English));
    assert_eq!(dict.detect_language("8iy["), None);
    assert_eq!(dict.detect_language("้ำสสน"), None);
    assert_eq!(dict.detect_language(""), None);
    assert_eq!(dict.detect_language("   "), None);
}

#[test]
fn test_detect_mismatched_language() {
    let thai_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/thai.txt"
    ));
    let eng_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/eng.txt"));

    let dict = DictionaryFilter::load_from_files(thai_path, eng_path)
        .expect("Failed to load dictionary asset files");

    let translator = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);

    // QWERTY "8iy[" translates to Thai "ครับ"
    assert_eq!(
        dict.detect_mismatched_language("8iy[", &translator),
        Some(Language::Thai)
    );

    // QWERTY "gxHo" translates to Thai "เป็น"
    assert_eq!(
        dict.detect_mismatched_language("gxHo", &translator),
        Some(Language::Thai)
    );

    // Kedmanee "้ำสสน" translates to English "hello"
    assert_eq!(
        dict.detect_mismatched_language("้ำสสน", &translator),
        Some(Language::English)
    );

    // Untranslated / random gibberish returns None
    assert_eq!(
        dict.detect_mismatched_language("qazwsxedc123", &translator),
        None
    );

    // Empty or whitespace returns None
    assert_eq!(dict.detect_mismatched_language("", &translator), None);
    assert_eq!(dict.detect_mismatched_language("   ", &translator), None);

    // Precedence test: words typed in their correct layout must NOT trigger mismatch detection
    assert_eq!(dict.detect_mismatched_language("me", &translator), None);
    assert_eq!(dict.detect_mismatched_language("if", &translator), None);
    assert_eq!(dict.detect_mismatched_language("สวัสดี", &translator), None);
    assert_eq!(dict.detect_mismatched_language("hello", &translator), None);
    assert_eq!(dict.detect_mismatched_language("ครับ", &translator), None);
}

#[test]
fn test_dictionary_load_str_with_bom_and_whitespace() {
    let thai_raw = "\u{feff}สวัสดี\r\n\r\n  เป็น  \r\nครับ\r\n";
    let eng_raw = "\u{feff}hello\r\n  WORLD  \r\n\r\n";

    let mut dict = DictionaryFilter::new();
    dict.load_thai_str(thai_raw);
    dict.load_eng_str(eng_raw);

    assert_eq!(dict.thai_trie().len(), 3);
    assert_eq!(dict.eng_trie().len(), 2);

    assert!(dict.contains_thai("สวัสดี"));
    assert!(dict.contains_thai("เป็น"));
    assert!(dict.contains_thai("ครับ"));

    assert!(dict.contains_english("hello"));
    assert!(dict.contains_english("world"));
    assert!(dict.contains_english("WORLD"));
}

#[test]
fn test_performance_constraints() {
    let thai_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/thai.txt"
    ));
    let eng_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/eng.txt"));

    let start_load = Instant::now();
    let dict = DictionaryFilter::load_from_files(thai_path, eng_path)
        .expect("Failed to load dictionary asset files");
    let load_duration = start_load.elapsed();

    // Constraint: Loading both dictionaries takes under 150ms
    assert!(
        load_duration.as_millis() < 150,
        "Loading duration {:?} exceeded 150ms ceiling",
        load_duration
    );

    let translator = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);

    // Constraint: Decision latency < 1ms per lookup on typical ~30-character buffer
    let sample_buffer = "8iy[";
    let start_lookup = Instant::now();
    const ITERATIONS: u32 = 10_000;
    for _ in 0..ITERATIONS {
        let res = dict.detect_mismatched_language(sample_buffer, &translator);
        assert_eq!(res, Some(Language::Thai));
    }
    let lookup_duration = start_lookup.elapsed();
    let avg_latency = lookup_duration / ITERATIONS;

    assert!(
        avg_latency.as_micros() < 1000,
        "Average decision latency {:?} exceeded 1ms budget",
        avg_latency
    );
}
