use rightlang_core::{
    layout::{DVORAK, KEDMANEE, PATTACHOTE, QWERTY},
    KeyTranslator, KeyboardLayout,
};

#[test]
fn test_qwerty_to_kedmanee_basic() {
    let translator = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);
    assert_eq!(translator.convert_str("8iy["), "ครับ");
    assert_eq!(translator.convert_str("l;ylfu"), "สวัสดี");
    assert_eq!(translator.convert_str("gxHo"), "เป็น");
}

#[test]
fn test_kedmanee_to_qwerty_basic() {
    let translator = KeyTranslator::new(KeyboardLayout::Kedmanee, KeyboardLayout::Qwerty);
    assert_eq!(translator.convert_str("ครับ"), "8iy[");
    assert_eq!(translator.convert_str("สวัสดี"), "l;ylfu");
    assert_eq!(translator.convert_str("เป็น"), "gxHo");
}

#[test]
fn test_dvorak_to_pattachote_basic() {
    let d_to_p = KeyTranslator::new(KeyboardLayout::Dvorak, KeyboardLayout::Pattachote);
    assert_eq!(d_to_p.convert_str("byi;"), "ครับ");
    assert_eq!(d_to_p.convert_str("ครับ"), "byi;");

    let p_to_d = KeyTranslator::new(KeyboardLayout::Pattachote, KeyboardLayout::Dvorak);
    assert_eq!(p_to_d.convert_str("ครับ"), "byi;");
    assert_eq!(p_to_d.convert_str("byi;"), "ครับ");
}

#[test]
fn test_qwerty_to_pattachote_basic() {
    let q_to_p = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Pattachote);
    assert_eq!(q_to_p.convert_str("ntgz"), "ครับ");
    assert_eq!(q_to_p.convert_str("ครับ"), "ntgz");

    let p_to_q = KeyTranslator::new(KeyboardLayout::Pattachote, KeyboardLayout::Qwerty);
    assert_eq!(p_to_q.convert_str("ครับ"), "ntgz");
    assert_eq!(p_to_q.convert_str("ntgz"), "ครับ");
}

#[test]
fn test_dvorak_to_kedmanee_basic() {
    let d_to_k = KeyTranslator::new(KeyboardLayout::Dvorak, KeyboardLayout::Kedmanee);
    // In Dvorak '8' maps to Kedmanee 'ค' (both at Row 0, Key 8)
    assert_eq!(d_to_k.convert_char('8'), 'ค');
    assert_eq!(d_to_k.convert_char('ค'), '8');
}

#[test]
fn test_shifted_keys() {
    let q_to_k = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);
    assert_eq!(q_to_k.convert_char('!'), '+');
    assert_eq!(q_to_k.convert_char('@'), '๑');
    assert_eq!(q_to_k.convert_char('#'), '๒');
    assert_eq!(q_to_k.convert_char('$'), '๓');

    let k_to_q = KeyTranslator::new(KeyboardLayout::Kedmanee, KeyboardLayout::Qwerty);
    assert_eq!(k_to_q.convert_char('+'), '!');
    assert_eq!(k_to_q.convert_char('๑'), '@');
    assert_eq!(k_to_q.convert_char('๒'), '#');
    assert_eq!(k_to_q.convert_char('๓'), '$');
}

#[test]
fn test_dvorak_pattachote_shifted() {
    let d_to_p = KeyTranslator::new(KeyboardLayout::Dvorak, KeyboardLayout::Pattachote);
    // Row 0 shifted: '!' -> '"', '@' -> '/'
    assert_eq!(d_to_p.convert_char('!'), '"');
    assert_eq!(d_to_p.convert_char('@'), '/');
    assert_eq!(d_to_p.convert_char('#'), ',');
    // Row 1 shifted: '<' -> 'ฤ', '>' -> 'ๆ'
    assert_eq!(d_to_p.convert_char('<'), 'ฤ');
    assert_eq!(d_to_p.convert_char('>'), 'ๆ');

    let p_to_d = KeyTranslator::new(KeyboardLayout::Pattachote, KeyboardLayout::Dvorak);
    assert_eq!(p_to_d.convert_char('"'), '!');
    assert_eq!(p_to_d.convert_char('/'), '@');
    assert_eq!(p_to_d.convert_char(','), '#');
    assert_eq!(p_to_d.convert_char('ฤ'), '<');
    assert_eq!(p_to_d.convert_char('ๆ'), '>');
}

#[test]
fn test_mixed_punctuation_and_untranslated() {
    let translator = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);
    // Spaces and emojis remain unchanged
    assert_eq!(translator.convert_str("8iy[ l;ylfu 😊"), "ครับ สวัสดี 😊");

    // Newlines and tabs remain unchanged
    assert_eq!(translator.convert_str("8iy[\n\tl;ylfu"), "ครับ\n\tสวัสดี");
}

#[test]
fn test_bidirectional_conversion_tolerance() {
    // When translating from QWERTY to Kedmanee, Thai text is also converted back to QWERTY if present
    let translator = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);
    assert_eq!(translator.convert_str("ครับ"), "8iy[");
}

#[test]
fn test_full_roundtrip_bijection() {
    let q_to_k = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);
    let k_to_q = KeyTranslator::new(KeyboardLayout::Kedmanee, KeyboardLayout::Qwerty);

    for (q_char, k_char) in QWERTY.chars().zip(KEDMANEE.chars()) {
        assert_eq!(q_to_k.convert_char(q_char), k_char);
        assert_eq!(k_to_q.convert_char(k_char), q_char);
    }

    assert_eq!(q_to_k.convert_str(QWERTY), KEDMANEE);
    assert_eq!(k_to_q.convert_str(KEDMANEE), QWERTY);

    // Verify constants exist and have expected non-zero lengths
    assert!(!DVORAK.is_empty());
    assert!(!PATTACHOTE.is_empty());
}
