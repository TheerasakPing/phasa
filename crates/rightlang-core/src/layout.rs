use std::collections::HashMap;

pub const QWERTY: &str = "1234567890-=qwertyuiop[]\\asdfghjkl;'zxcvbnm,./!@#$%^&*()_+QWERTYUIOP{}|ASDFGHJKL:\"ZXCVBNM<>?";
pub const KEDMANEE: &str =
    "ๅ/_ภถุึคตจขชๆไำพะัีรนยบลฃฟหกดเ้่าสวงผปแอิืทมใฝ+๑๒๓๔ู฿๕๖๗๘๙๐\"ฎฑธํ๊ณฯญฐ,ฅฤฆฏโฌ็๋ษศซ.()ฉฮฺ์?ฒฬฦ";
pub const DVORAK: &str =
    "1234567890[]',.pyfgcrl/=\\aoeuidhtns-;qjkxbmwvz!@#$%^&*(){}\"<>PYFGCRL?+|AOEUIDHTNS_:QJKXBMWVZ";
pub const PATTACHOTE: &str =
    "๒๓๔๕ู๗๘๙๐๑๖็ตยอร่ดมวแใฌฃ้ทงกัีานเไขบปลหิคสะจพ\"/,?ุ_.()-%๊ฤๆญษึฝซถฒฯภฅ๋ธำณ์ืผชโฆฑฎฏฐภัศฮฟฉฬ";

pub const QWERTY_ROWS: [&str; 8] = [
    "1234567890-=",
    "qwertyuiop[]\\",
    "asdfghjkl;'",
    "zxcvbnm,./",
    "!@#$%^&*()_+",
    "QWERTYUIOP{}|",
    "ASDFGHJKL:\"",
    "ZXCVBNM<>?",
];

pub const DVORAK_ROWS: [&str; 8] = [
    "1234567890[]",
    "',.pyfgcrl/=\\",
    "aoeuidhtns-",
    ";qjkxbmwvz",
    "!@#$%^&*(){}",
    "\"<>PYFGCRL?+|",
    "AOEUIDHTNS_",
    ":QJKXBMWVZ",
];

pub const KEDMANEE_ROWS: [&str; 8] = [
    "ๅ/_ภถุึคตจขช",
    "ๆไำพะัีรนยบลฃ",
    "ฟหกดเ้่าสวง",
    "ผปแอิืทมใฝ",
    "+๑๒๓๔ู฿๕๖๗๘๙",
    "๐\"ฎฑธํ๊ณฯญฐ,ฅ",
    "ฤฆฏโฌ็๋ษศซ.",
    "()ฉฮฺ์?ฒฬฦ",
];

pub const PATTACHOTE_ROWS: [&str; 8] = [
    "๒๓๔๕ู๗๘๙๐๑๖",
    "็ตยอร่ดมวแใฌฃ",
    "้ทงกัีานเไข",
    "บปลหิคสะจพ",
    "\"/,?ุ_.()-%",
    "๊ฤๆญษึฝซถฒฯภฅ",
    "๋ธำณ์ืผชโฆฑ",
    "ฎฏฐภัศฮฟฉฬ",
];

/// Returns the 8 physical keyboard layout rows (4 unshifted, 4 shifted) for a given layout.
pub fn layout_rows(layout: KeyboardLayout) -> &'static [&'static str; 8] {
    match layout {
        KeyboardLayout::Qwerty => &QWERTY_ROWS,
        KeyboardLayout::Dvorak => &DVORAK_ROWS,
        KeyboardLayout::Kedmanee => &KEDMANEE_ROWS,
        KeyboardLayout::Pattachote => &PATTACHOTE_ROWS,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyboardLayout {
    Kedmanee,
    Pattachote,
    Qwerty,
    Dvorak,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyTranslator {
    pub source: KeyboardLayout,
    pub target: KeyboardLayout,
    pub forward_map: HashMap<char, char>,
    pub reverse_map: HashMap<char, char>,
}

impl KeyTranslator {
    pub fn new(source: KeyboardLayout, target: KeyboardLayout) -> Self {
        let mut forward_map = HashMap::new();
        let mut reverse_map = HashMap::new();

        let s_rows = layout_rows(source);
        let t_rows = layout_rows(target);

        for (s_row, t_row) in s_rows.iter().zip(t_rows.iter()) {
            for (s, t) in s_row.chars().zip(t_row.chars()) {
                // Preserve unshifted primary mappings encountered first before shifted duplicates
                forward_map.entry(s).or_insert(t);
                reverse_map.entry(t).or_insert(s);
            }
        }

        Self {
            source,
            target,
            forward_map,
            reverse_map,
        }
    }

    /// Converts a single character.
    ///
    /// If the character is in the forward map (source -> target), it is converted.
    /// Otherwise, if it is in the reverse map (target -> source), it is converted back,
    /// enabling bidirectional translation of mixed or mismatched input buffers.
    /// Characters not present in either layout map are preserved as-is.
    pub fn convert_char(&self, ch: char) -> char {
        self.forward_map
            .get(&ch)
            .or_else(|| self.reverse_map.get(&ch))
            .copied()
            .unwrap_or(ch)
    }

    /// Converts an input string according to the configured layouts.
    pub fn convert_str(&self, input: &str) -> String {
        input.chars().map(|ch| self.convert_char(ch)).collect()
    }

    pub fn source(&self) -> KeyboardLayout {
        self.source
    }

    pub fn target(&self) -> KeyboardLayout {
        self.target
    }

    pub fn forward_map(&self) -> &HashMap<char, char> {
        &self.forward_map
    }

    pub fn reverse_map(&self) -> &HashMap<char, char> {
        &self.reverse_map
    }
}
