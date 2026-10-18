use std::collections::HashMap;

pub const QWERTY: &str = "1234567890-=qwertyuiop[]\\asdfghjkl;'zxcvbnm,./!@#$%^&*()_+QWERTYUIOP{}|ASDFGHJKL:\"ZXCVBNM<>?";
pub const KEDMANEE: &str =
    "ๅ/_ภถุึคตจขชๆไำพะัีรนยบลฃฟหกดเ้่าสวงผปแอิืทมใฝ+๑๒๓๔ู฿๕๖๗๘๙๐\"ฎฑธํ๊ณฯญฐ,ฅฤฆฏโฌ็๋ษศซ.()ฉฮฺ์?ฒฬฦ";

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

        match (source, target) {
            (KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee) => {
                for (q, k) in QWERTY.chars().zip(KEDMANEE.chars()) {
                    forward_map.insert(q, k);
                    reverse_map.insert(k, q);
                }
            }
            (KeyboardLayout::Kedmanee, KeyboardLayout::Qwerty) => {
                for (q, k) in QWERTY.chars().zip(KEDMANEE.chars()) {
                    forward_map.insert(k, q);
                    reverse_map.insert(q, k);
                }
            }
            _ => {}
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
