//! Main keyboard event worker state machine and correction pipeline for RightLang.

use std::path::{Path, PathBuf};
use std::time::Instant;

use rightlang_core::dictionary::Language;
use rightlang_core::layout::{KeyTranslator, KeyboardLayout};
use rightlang_core::DictionaryFilter;
use rightlang_macos::dispatcher::KeyDispatcher;
use rightlang_macos::event::{KeyEvent, KeyEventType};
use rightlang_macos::tis::InputSourceManager;

use crate::config::{AppConfig, CorrectionMode};
use crate::rules::ApplicationRuleManager;

/// Record of an executed text correction event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionRecord {
    /// Original mis-typed string before conversion.
    pub original: String,
    /// Converted replacement string dispatched to the application.
    pub replacement: String,
    /// Target language that was detected and switched to.
    pub target_language: Language,
    /// Timestamp when the correction occurred.
    pub timestamp: Instant,
}

/// The core worker state machine that consumes raw keyboard events, maintains the
/// typing buffer, detects language mismatches, and orchestrates synthetic corrections.
pub struct KeyWorker {
    config: AppConfig,
    rule_manager: ApplicationRuleManager,
    dictionary: DictionaryFilter,
    translator: KeyTranslator,
    dispatcher: KeyDispatcher,
    input_source_manager: InputSourceManager,
    buffer: String,
    last_correction: Option<CorrectionRecord>,
    active_app: Option<String>,
    user_thai_dict_path: PathBuf,
    user_eng_dict_path: PathBuf,
}

impl KeyWorker {
    /// Creates a new `KeyWorker` configured with application settings and dual-language dictionaries.
    pub fn new(config: AppConfig, mut dictionary: DictionaryFilter) -> Self {
        let rule_manager = ApplicationRuleManager::from_config(&config);
        let translator = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);
        let dispatcher = KeyDispatcher::new(config.key_delay_us);
        let input_source_manager = InputSourceManager::new();
        let user_thai_dict_path = AppConfig::default_user_thai_dict_path();
        let user_eng_dict_path = AppConfig::default_user_eng_dict_path();

        let _ = dictionary.load_user_words(&user_thai_dict_path, &user_eng_dict_path);

        Self {
            config,
            rule_manager,
            dictionary,
            translator,
            dispatcher,
            input_source_manager,
            buffer: String::new(),
            last_correction: None,
            active_app: None,
            user_thai_dict_path,
            user_eng_dict_path,
        }
    }

    /// Sets custom file paths for user custom dictionaries (useful for isolated tests).
    pub fn set_user_dict_paths<P: AsRef<Path>>(&mut self, thai_path: P, eng_path: P) {
        self.user_thai_dict_path = thai_path.as_ref().to_path_buf();
        self.user_eng_dict_path = eng_path.as_ref().to_path_buf();
        let _ = self.dictionary.load_user_words(&self.user_thai_dict_path, &self.user_eng_dict_path);
    }

    /// Returns the active paths for custom Thai and English user dictionaries.
    pub fn user_dict_paths(&self) -> (&Path, &Path) {
        (&self.user_thai_dict_path, &self.user_eng_dict_path)
    }

    /// Learns a word into the user dictionary and persists it to disk if `auto_learn` is enabled.
    pub fn learn_word(&mut self, word: &str, lang: Language) -> bool {
        let added = self.dictionary.add_user_word(word, lang);
        if added && self.config.auto_learn {
            let _ = self.dictionary.save_user_words(&self.user_thai_dict_path, &self.user_eng_dict_path);
        }
        added
    }

    /// Process a single incoming `KeyEvent` from the `EventTapListener` channel.
    ///
    /// Returns `Some(CorrectionRecord)` if a correction was triggered, or `None` otherwise.
    pub fn handle_event(&mut self, event: &KeyEvent) -> Option<CorrectionRecord> {
        // Only process KeyDown events
        if event.event_type != KeyEventType::KeyDown {
            return None;
        }

        // Ignore synthetic events to avoid feedback loops
        if event.is_synthetic {
            return None;
        }

        // Check for Undo shortcuts: Cmd+Shift+Z or Ctrl+Option+Z
        // macOS kCGEventFlagMaskShift = 0x00020000, Control = 0x00040000,
        // Alternate (Option) = 0x00080000, Command = 0x00100000
        const SHIFT_FLAG: u64 = 0x00020000;
        const CTRL_FLAG: u64 = 0x00040000;
        const ALT_FLAG: u64 = 0x00080000;
        const CMD_FLAG: u64 = 0x00100000;

        let is_z = event.key_code == 0x06
            || event
                .characters
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case("z") || s == "\u{1a}")
                .unwrap_or(false);

        let is_cmd_shift_z =
            is_z && ((event.flags & CMD_FLAG) != 0) && ((event.flags & SHIFT_FLAG) != 0);
        let is_ctrl_opt_z =
            is_z && ((event.flags & CTRL_FLAG) != 0) && ((event.flags & ALT_FLAG) != 0);

        if is_cmd_shift_z || is_ctrl_opt_z {
            self.undo();
            return None;
        }

        // Backspace key (macOS key code 0x33 = 51)
        if event.key_code == 0x33 {
            self.buffer.pop();
            return None;
        }

        // General Command or Control modifier keys active (e.g. shortcuts like Cmd+C, Cmd+V, Ctrl+C)
        if (event.flags & (CMD_FLAG | CTRL_FLAG)) != 0 {
            self.clear_buffer();
            return None;
        }

        // Navigation, Enter, Tab, and Escape keys clear the current buffer
        // Enter: 0x24 (Return), 0x4C (NumPad Enter)
        // Tab: 0x30, Escape: 0x35
        // Navigation: Home: 0x73, PageUp: 0x74, End: 0x77, PageDown: 0x79,
        //             Left: 0x7B, Right: 0x7C, Down: 0x7D, Up: 0x7E
        match event.key_code {
            0x24 | 0x4C | 0x30 | 0x35 | 0x73 | 0x74 | 0x77 | 0x79 | 0x7B | 0x7C | 0x7D | 0x7E => {
                self.clear_buffer();
                return None;
            }
            _ => {}
        }

        // Space key (key code 0x31 = 49) or character " "
        let is_space = event.key_code == 0x31
            || event
                .characters
                .as_deref()
                .map(|s| s == " ")
                .unwrap_or(false);

        if is_space {
            let current_app_id = self.resolve_active_app();
            let mode = self.rule_manager.mode_for_app(&current_app_id);
            if mode == CorrectionMode::Auto {
                let result = self.evaluate_and_correct_internal(true);
                if result.is_some() {
                    return result;
                }
            }
            self.clear_buffer();
            return None;
        }

        // Character keys: append to buffer
        if let Some(ref chars) = event.characters {
            // Check for control / whitespace characters
            if chars.chars().any(|c| c.is_control() || c.is_whitespace()) {
                self.clear_buffer();
                return None;
            }

            self.buffer.push_str(chars);

            // Guard against unbounded buffer growth
            if self.buffer.len() > 128 {
                self.clear_buffer();
            }
        }

        None
    }

    /// Check buffer and trigger correction if mismatched language is detected.
    /// Does not include trailing space offset (suitable for manual hotkey invocation).
    pub fn evaluate_and_correct(&mut self) -> Option<CorrectionRecord> {
        self.evaluate_and_correct_internal(false)
    }

    /// Check buffer and trigger correction with optional space delimiter offset.
    pub fn evaluate_and_correct_with_space(&mut self, is_space: bool) -> Option<CorrectionRecord> {
        self.evaluate_and_correct_internal(is_space)
    }

    fn evaluate_and_correct_internal(&mut self, is_space: bool) -> Option<CorrectionRecord> {
        let current_app_id = self.resolve_active_app();
        let mode = self.rule_manager.mode_for_app(&current_app_id);

        if mode == CorrectionMode::Disabled {
            self.clear_buffer();
            return None;
        }

        if self.buffer.trim().is_empty() {
            return None;
        }

        let detected = self
            .dictionary
            .detect_mismatched_language(&self.buffer, &self.translator);

        if let Some(target_lang) = detected {
            let mut replacement = self.translator.convert_str(&self.buffer);
            let mut backspace_count = self.buffer.encode_utf16().count();

            // When triggered by space delimiter, macOS already passed the trailing space through,
            // so we backspace the word + 1 space, and append a space to the replacement.
            if is_space {
                backspace_count += 1;
                replacement.push(' ');
            }

            // Dispatch synthetic backspaces and replacement text
            let _ = self.dispatcher.replace_text(
                backspace_count,
                &replacement,
                Some(self.config.key_delay_us),
            );

            // Switch macOS input source to target language
            match target_lang {
                Language::Thai => {
                    let _ = self.input_source_manager.switch_to_thai();
                }
                Language::English => {
                    let _ = self.input_source_manager.switch_to_english();
                }
            }

            let record = CorrectionRecord {
                original: self.buffer.clone(),
                replacement: replacement.clone(),
                target_language: target_lang,
                timestamp: Instant::now(),
            };

            // If auto_learn is enabled, automatically learn the valid corrected word
            if self.config.auto_learn {
                self.learn_word(replacement.trim(), target_lang);
            }

            self.last_correction = Some(record.clone());
            self.buffer.clear();
            Some(record)
        } else {
            None
        }
    }

    /// Undo last correction if available, restoring the original buffer and keyboard layout.
    pub fn undo(&mut self) -> bool {
        if let Some(record) = self.last_correction.take() {
            let backspace_count = record.replacement.encode_utf16().count();
            let _ = self.dispatcher.replace_text(
                backspace_count,
                &record.original,
                Some(self.config.key_delay_us),
            );

            match record.target_language {
                Language::Thai => {
                    let _ = self.input_source_manager.switch_to_english();
                }
                Language::English => {
                    let _ = self.input_source_manager.switch_to_thai();
                }
            }

            // If auto_learn is enabled, learning the original word in its source language
            // ensures the user's intended term is remembered and not erroneously converted again.
            if self.config.auto_learn {
                let source_lang = match record.target_language {
                    Language::Thai => Language::English,
                    Language::English => Language::Thai,
                };
                self.learn_word(record.original.trim(), source_lang);
            }

            self.buffer = record.original;
            true
        } else {
            false
        }
    }

    /// Returns a reference to the current buffer string.
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// Clears the typing buffer.
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    /// Updates the global correction mode.
    pub fn set_mode(&mut self, mode: CorrectionMode) {
        self.config.mode = mode;
        self.rule_manager.set_default_mode(mode);
    }

    /// Returns the last correction record, if any.
    pub fn last_correction(&self) -> Option<&CorrectionRecord> {
        self.last_correction.as_ref()
    }

    /// Sets an explicit active application bundle ID override (useful for testing).
    pub fn set_active_app(&mut self, app: Option<String>) {
        self.active_app = app;
    }

    /// Returns the currently configured active application override, if any.
    pub fn active_app(&self) -> Option<&str> {
        self.active_app.as_deref()
    }

    /// Returns a reference to the active configuration.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Returns a mutable reference to the active configuration.
    pub fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    /// Returns a reference to the application rule manager.
    pub fn rule_manager(&self) -> &ApplicationRuleManager {
        &self.rule_manager
    }

    /// Returns a mutable reference to the application rule manager.
    pub fn rule_manager_mut(&mut self) -> &mut ApplicationRuleManager {
        &mut self.rule_manager
    }

    /// Returns a reference to the dictionary filter.
    pub fn dictionary(&self) -> &DictionaryFilter {
        &self.dictionary
    }

    /// Returns a mutable reference to the dictionary filter.
    pub fn dictionary_mut(&mut self) -> &mut DictionaryFilter {
        &mut self.dictionary
    }

    /// Returns a reference to the key translator.
    pub fn translator(&self) -> &KeyTranslator {
        &self.translator
    }

    /// Returns a reference to the key dispatcher.
    pub fn dispatcher(&self) -> &KeyDispatcher {
        &self.dispatcher
    }

    /// Returns a reference to the input source manager.
    pub fn input_source_manager(&self) -> &InputSourceManager {
        &self.input_source_manager
    }

    fn resolve_active_app(&self) -> String {
        if let Some(ref app) = self.active_app {
            return app.clone();
        }
        #[cfg(test)]
        {
            if let Some(front) = crate::rules::get_frontmost_app_bundle_id() {
                if self.rule_manager.mode_for_app(&front) != CorrectionMode::Disabled {
                    return front;
                }
            }
            "com.rightlang.testapp".to_string()
        }
        #[cfg(not(test))]
        {
            crate::rules::get_frontmost_app_bundle_id().unwrap_or_default()
        }
    }
}

/// Loads Thai and English dictionaries from user configuration directory, relative paths,
/// app bundle resources, or embedded binary fallbacks.
pub fn load_dictionaries() -> DictionaryFilter {
    let mut dict = None;

    // 1. Check ~/.config/rightlang/
    if let Some(home) = dirs::home_dir() {
        let dir = home.join(".config").join("rightlang");
        let thai = dir.join("thai.txt");
        let eng = dir.join("eng.txt");
        if thai.exists() && eng.exists() {
            if let Ok(d) = DictionaryFilter::load_from_files(&thai, &eng) {
                dict = Some(d);
            }
        }
    }

    // 2. Check macOS app bundle Resources directory
    if dict.is_none() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(macos_dir) = exe_path.parent() {
                if let Some(contents_dir) = macos_dir.parent() {
                    let res_dir = contents_dir.join("Resources");
                    let thai = res_dir.join("thai.txt");
                    let eng = res_dir.join("eng.txt");
                    if thai.exists() && eng.exists() {
                        if let Ok(d) = DictionaryFilter::load_from_files(&thai, &eng) {
                            dict = Some(d);
                        }
                    }
                }
            }
        }
    }

    // 3. Check relative candidate paths
    if dict.is_none() {
        let candidates = [
            ("assets/thai.txt", "assets/eng.txt"),
            ("../assets/thai.txt", "../assets/eng.txt"),
            ("../../assets/thai.txt", "../../assets/eng.txt"),
        ];
        for (thai_p, eng_p) in candidates {
            let thai = std::path::Path::new(thai_p);
            let eng = std::path::Path::new(eng_p);
            if thai.exists() && eng.exists() {
                if let Ok(d) = DictionaryFilter::load_from_files(thai, eng) {
                    dict = Some(d);
                    break;
                }
            }
        }
    }

    // 4. Fallback to embedded dictionary assets
    let mut loaded = dict.unwrap_or_else(|| {
        let mut d = DictionaryFilter::new();
        d.load_thai_str(include_str!("../../../assets/thai.txt"));
        d.load_eng_str(include_str!("../../../assets/eng.txt"));
        d.shrink_to_fit();
        d
    });

    // 5. Load custom user words from ~/.config/rightlang/user.th.txt and user.en.txt
    let user_thai = AppConfig::default_user_thai_dict_path();
    let user_eng = AppConfig::default_user_eng_dict_path();
    let _ = loaded.load_user_words(&user_thai, &user_eng);

    loaded
}
