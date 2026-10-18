use std::fs;
use std::io;
use std::path::Path;

use crate::layout::KeyTranslator;
use crate::trie::CompactTrie;

/// Supported languages for detection and dictionary filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Thai,
    English,
}

/// Dual-dictionary filter holding compact tries for Thai and English.
#[derive(Debug, Clone, Default)]
pub struct DictionaryFilter {
    thai_trie: CompactTrie,
    eng_trie: CompactTrie,
}

impl DictionaryFilter {
    /// Creates a new, empty `DictionaryFilter`.
    pub fn new() -> Self {
        Self {
            thai_trie: CompactTrie::new(),
            eng_trie: CompactTrie::new(),
        }
    }

    /// Loads Thai words from a string (newline-separated, handles BOM/CRLF/whitespace).
    pub fn load_thai_str(&mut self, content: &str) {
        for line in content.lines() {
            let trimmed = line.trim().trim_matches('\u{feff}');
            if !trimmed.is_empty() {
                self.thai_trie.insert(trimmed);
            }
        }
        self.thai_trie.shrink_to_fit();
    }

    /// Loads English words from a string (newline-separated, handles BOM/CRLF/whitespace, converted to lowercase).
    pub fn load_eng_str(&mut self, content: &str) {
        for line in content.lines() {
            let trimmed = line.trim().trim_matches('\u{feff}');
            if !trimmed.is_empty() {
                self.eng_trie.insert(&trimmed.to_lowercase());
            }
        }
        self.eng_trie.shrink_to_fit();
    }

    /// Loads Thai and English dictionaries from specified file paths.
    pub fn load_from_files<P: AsRef<Path>>(thai_path: P, eng_path: P) -> io::Result<Self> {
        let thai_content = fs::read_to_string(thai_path)?;
        let eng_content = fs::read_to_string(eng_path)?;

        let mut filter = Self::new();
        filter.load_thai_str(&thai_content);
        filter.load_eng_str(&eng_content);
        filter.shrink_to_fit();
        Ok(filter)
    }

    /// Shrinks internal memory allocations for both Thai and English tries.
    pub fn shrink_to_fit(&mut self) {
        self.thai_trie.shrink_to_fit();
        self.eng_trie.shrink_to_fit();
    }

    /// Checks if a word exists in the Thai dictionary.
    pub fn contains_thai(&self, word: &str) -> bool {
        self.thai_trie.contains(word)
    }

    /// Checks if a word exists in the English dictionary (case-insensitive).
    pub fn contains_english(&self, word: &str) -> bool {
        if word.chars().any(|c| c.is_uppercase()) {
            self.eng_trie.contains(&word.to_lowercase())
        } else {
            self.eng_trie.contains(word)
        }
    }

    /// Checks if a prefix exists in the Thai dictionary.
    pub fn is_thai_prefix(&self, prefix: &str) -> bool {
        self.thai_trie.is_prefix(prefix)
    }

    /// Checks if a prefix exists in the English dictionary (case-insensitive).
    pub fn is_eng_prefix(&self, prefix: &str) -> bool {
        if prefix.chars().any(|c| c.is_uppercase()) {
            self.eng_trie.is_prefix(&prefix.to_lowercase())
        } else {
            self.eng_trie.is_prefix(prefix)
        }
    }

    /// Checks if a word exists in either the Thai or English dictionary.
    pub fn contains(&self, word: &str) -> bool {
        self.contains_thai(word) || self.contains_english(word)
    }

    /// Checks if a prefix exists in either the Thai or English dictionary.
    pub fn is_prefix(&self, prefix: &str) -> bool {
        self.is_thai_prefix(prefix) || self.is_eng_prefix(prefix)
    }

    /// Directly checks what language the raw buffer is in.
    ///
    /// Returns `Some(Language::Thai)` if present in the Thai dictionary,
    /// `Some(Language::English)` if present in the English dictionary,
    /// or `None` if unrecognized.
    pub fn detect_language(&self, buffer: &str) -> Option<Language> {
        let trimmed = buffer.trim();
        if trimmed.is_empty() {
            return None;
        }

        if self.contains_thai(trimmed) {
            return Some(Language::Thai);
        }
        if self.contains_english(trimmed) {
            return Some(Language::English);
        }

        None
    }

    /// Evaluates an input buffer to detect if it was typed using the wrong keyboard layout.
    ///
    /// If the buffer is already a valid word in its current script/language, returns `None`
    /// (no mismatch).
    /// Only if the buffer is not valid in its current script, but its converted layout
    /// counterpart via `KeyTranslator` is valid, returns `Some(target_language)`.
    pub fn detect_mismatched_language(
        &self,
        buffer: &str,
        translator: &KeyTranslator,
    ) -> Option<Language> {
        let trimmed = buffer.trim();
        if trimmed.is_empty() {
            return None;
        }

        // If buffer is already valid in its current script/language, return None (no mismatch!)
        if self.detect_language(trimmed).is_some() {
            return None;
        }

        // Only if buffer is NOT valid in its current script, but converted IS valid, return Some(target_language)
        let converted = translator.convert_str(trimmed);
        self.detect_language(&converted)
    }

    /// Returns a reference to the Thai trie.
    pub fn thai_trie(&self) -> &CompactTrie {
        &self.thai_trie
    }

    /// Returns a reference to the English trie.
    pub fn eng_trie(&self) -> &CompactTrie {
        &self.eng_trie
    }

    /// Returns a mutable reference to the Thai trie.
    pub fn thai_trie_mut(&mut self) -> &mut CompactTrie {
        &mut self.thai_trie
    }

    /// Returns a mutable reference to the English trie.
    pub fn eng_trie_mut(&mut self) -> &mut CompactTrie {
        &mut self.eng_trie
    }
}
