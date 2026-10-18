use rightlang_core::distance::UNLINKED_DISTANCE;
use rightlang_core::KeyDistanceMatrix;
use std::path::Path;
use std::time::Instant;

#[test]
fn test_adjacent_direct_pairs() {
    let matrix = KeyDistanceMatrix::new();
    assert_eq!(matrix.distance('q', 'w'), 1);
    assert_eq!(matrix.distance('w', 'q'), 1);
    assert_eq!(matrix.distance('1', '2'), 1);
    assert_eq!(matrix.distance('2', '1'), 1);
    // Case-insensitivity
    assert_eq!(matrix.distance('Q', 'W'), 1);
    assert_eq!(matrix.distance('q', 'W'), 1);
    assert_eq!(matrix.distance('W', 'q'), 1);
}

#[test]
fn test_non_adjacent_pairs() {
    let matrix = KeyDistanceMatrix::new();
    let d_qp = matrix.distance('q', 'p');
    assert!(
        d_qp > 1,
        "Expected distance between 'q' and 'p' > 1, got {}",
        d_qp
    );

    let d_19 = matrix.distance('1', '9');
    assert!(
        d_19 > 1,
        "Expected distance between '1' and '9' > 1, got {}",
        d_19
    );
}

#[test]
fn test_are_adjacent() {
    let matrix = KeyDistanceMatrix::new();
    assert!(matrix.are_adjacent('a', 's'));
    assert!(matrix.are_adjacent('s', 'a'));
    assert!(!matrix.are_adjacent('a', 'l'));
    assert!(!matrix.are_adjacent('l', 'a'));

    // Case-insensitivity
    assert!(matrix.are_adjacent('A', 'S'));
    assert!(matrix.are_adjacent('a', 'S'));

    // Identical characters are distance 0, not adjacent
    assert!(!matrix.are_adjacent('a', 'a'));
    assert_eq!(matrix.distance('a', 'a'), 0);
}

#[test]
fn test_load_from_assets() {
    let manifest_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/adjacent-column.txt"
    );
    let asset_path = if Path::new(manifest_path).exists() {
        Path::new(manifest_path)
    } else {
        Path::new("assets/adjacent-column.txt")
    };
    let matrix =
        KeyDistanceMatrix::load_from_file(asset_path).expect("Failed to load adjacent-column.txt");
    assert_eq!(matrix.distance('q', 'w'), 1);
    assert!(matrix.are_adjacent('a', 's'));
}

#[test]
fn test_fuzzy_match() {
    let matrix = KeyDistanceMatrix::new();

    // Exact match
    assert!(matrix.fuzzy_match("hello", "hello", 0));
    assert!(matrix.fuzzy_match("hello", "hello", 1));

    // Case-insensitive match
    assert!(matrix.fuzzy_match("Hello", "hello", 0));

    // 1 adjacent-key typo: 'r' is adjacent to 'e' in QWERTY row (we, er)
    assert!(matrix.fuzzy_match("hrllo", "hello", 1));
    assert!(!matrix.fuzzy_match("hrllo", "hello", 0));

    // 1 non-adjacent typo: 'z' and 'e' are not adjacent
    // Adjacent typo costs 1, non-adjacent costs 2
    assert!(!matrix.fuzzy_match("hzllo", "hello", 1));
    assert!(matrix.fuzzy_match("hzllo", "hello", 2));

    // 2 adjacent typos: 'r' vs 'e' (1) and 'p' vs 'o' (1) -> total cost 2
    assert!(!matrix.fuzzy_match("hrllp", "hello", 1));
    assert!(matrix.fuzzy_match("hrllp", "hello", 2));

    // Unrelated words exceed max_cost
    assert!(!matrix.fuzzy_match("world", "hello", 2));

    // Length difference handling (insert/delete)
    assert!(matrix.fuzzy_match("helo", "hello", 1));
    assert!(!matrix.fuzzy_match("helo", "hello", 0));
}

#[test]
fn test_thai_adjacent_keys() {
    let matrix = KeyDistanceMatrix::new();
    // In Kedmanee / adjacent-column.txt: 'ฟ' and 'ห' are adjacent
    assert!(matrix.are_adjacent('ฟ', 'ห'));
    assert_eq!(matrix.distance('ฟ', 'ห'), 1);

    // 'ฟ' and 'ง' are in the same row but far apart
    assert!(!matrix.are_adjacent('ฟ', 'ง'));
    assert!(matrix.distance('ฟ', 'ง') > 1);
}

#[test]
fn test_custom_pairs_str() {
    let custom = "\u{feff}ab\r\n# Comment\r\nbc\r\n// Another comment\r\ncd\r\n\r\n";
    let matrix = KeyDistanceMatrix::from_adjacent_pairs_str(custom);
    assert_eq!(matrix.distance('a', 'b'), 1);
    assert_eq!(matrix.distance('b', 'c'), 1);
    assert_eq!(matrix.distance('a', 'c'), 2);
    assert_eq!(matrix.distance('a', 'd'), 3);
    assert!(matrix.are_adjacent('a', 'b'));
    assert!(!matrix.are_adjacent('a', 'c'));

    // Alias from_adjacent_pairs
    let matrix2 = KeyDistanceMatrix::from_adjacent_pairs(custom);
    assert_eq!(matrix2.distance('a', 'b'), 1);
}

#[test]
fn test_unlinked_and_unknown_characters() {
    let matrix = KeyDistanceMatrix::new();

    // Identical unknown characters have distance 0
    assert_eq!(matrix.distance('~', '~'), 0);
    assert_eq!(matrix.distance(' ', ' '), 0);

    // Unlinked characters
    let unlinked_dist = matrix.distance('q', '~');
    assert_eq!(unlinked_dist, UNLINKED_DISTANCE);
    assert!(!matrix.are_adjacent('q', '~'));

    // Empty matrix
    let empty_matrix = KeyDistanceMatrix::empty();
    assert_eq!(empty_matrix.distance('a', 'a'), 0);
    assert_eq!(empty_matrix.distance('a', 'b'), UNLINKED_DISTANCE);
    assert!(!empty_matrix.are_adjacent('a', 'b'));
}

#[test]
fn test_lookup_performance() {
    let matrix = KeyDistanceMatrix::new();
    const ITERATIONS: u32 = 100_000;

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _ = matrix.distance('q', 'w');
        let _ = matrix.distance('a', 's');
        let _ = matrix.distance('ฟ', 'ห');
    }
    let duration = start.elapsed();
    let avg_ns = duration.as_nanos() / (ITERATIONS as u128 * 3);

    #[cfg(debug_assertions)]
    let max_budget_ns = 250;
    #[cfg(not(debug_assertions))]
    let max_budget_ns = 100;

    assert!(
        avg_ns < max_budget_ns,
        "Average lookup latency {:?} ns exceeded {}ns budget",
        avg_ns,
        max_budget_ns
    );
}
