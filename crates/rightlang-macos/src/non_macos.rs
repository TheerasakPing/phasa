//! Cross-platform stubs for non-macOS operating systems (Linux, Windows).

pub mod clipboard {
    pub fn get_clipboard_text() -> Option<String> {
        None
    }

    pub fn set_clipboard_text(_text: &str) {}
}

pub mod dispatcher {
    #[derive(Debug, thiserror::Error)]
    pub enum DispatchError {
        #[error("Platform keyboard dispatch unsupported on non-macOS")]
        Unsupported,
    }

    pub struct KeyDispatcher {
        _delay_us: u64,
    }

    impl KeyDispatcher {
        pub fn new(delay_us: u64) -> Self {
            Self { _delay_us: delay_us }
        }

        pub fn replace_text(
            &self,
            _backspaces: usize,
            _replacement: &str,
            _key_delay_us: Option<u64>,
        ) -> Result<(), DispatchError> {
            Ok(())
        }

        pub fn send_key_combination(
            &self,
            _key_code: u16,
            _flags: u64,
            _delay_us: u64,
        ) -> Result<(), DispatchError> {
            Ok(())
        }
    }
}

pub mod event {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum KeyEventType {
        KeyDown,
        KeyUp,
        FlagsChanged,
        Other,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct KeyEvent {
        pub event_type: KeyEventType,
        pub key_code: u16,
        pub flags: u64,
        pub characters: Option<String>,
        pub is_synthetic: bool,
    }

    pub const RIGHTLANG_EVENT_TAG: i64 = 0x524c414e47;

    pub fn is_rightlang_event(_event: &KeyEvent) -> bool {
        false
    }

    pub fn tag_event_as_rightlang() {}
    pub fn is_event_tagged_as_rightlang() -> bool {
        false
    }
    pub fn extract_characters(_event: &KeyEvent) -> Option<String> {
        None
    }
}

pub mod ffi {}

pub mod hook {
    use crossbeam_channel::Sender;
    use super::event::KeyEvent;

    #[derive(Debug, thiserror::Error)]
    pub enum HookError {
        #[error("Event tap is unsupported on non-macOS")]
        Unsupported,
    }

    pub fn is_accessibility_trusted(_prompt: bool) -> bool {
        true
    }

    pub struct EventTapListener;

    impl EventTapListener {
        pub fn new(_event_tx: Sender<KeyEvent>) -> Result<Self, HookError> {
            Err(HookError::Unsupported)
        }

        pub fn start_with_run_loop(
            &self,
            _run_loop: *mut std::ffi::c_void,
        ) -> Result<*mut std::ffi::c_void, HookError> {
            Err(HookError::Unsupported)
        }

        pub fn stop(&self) {}
        pub fn is_running(&self) -> bool { false }
        pub fn is_valid(&self) -> bool { false }
        pub fn is_enabled(&self) -> bool { false }
        pub fn re_enable(&self) -> bool { false }
    }
}

pub mod tis {
    #[derive(Debug, thiserror::Error)]
    pub enum TisError {
        #[error("Input source switching unsupported on non-macOS")]
        Unsupported,
    }

    #[derive(Debug, Clone)]
    pub struct InputSourceInfo {
        pub id: String,
        pub name: String,
        pub languages: Vec<String>,
    }

    #[derive(Debug, Clone)]
    pub struct InputSourceManager;

    impl InputSourceManager {
        pub fn new() -> Self {
            Self
        }

        pub fn get_current_source_id(&self) -> Result<String, TisError> {
            Ok("standard".to_string())
        }

        pub fn list_keyboard_input_sources(&self) -> Result<Vec<InputSourceInfo>, TisError> {
            Ok(vec![])
        }

        pub fn select_input_source(&self, _source_id: &str) -> Result<(), TisError> {
            Ok(())
        }

        pub fn switch_to_thai(&self) -> Result<(), TisError> {
            Ok(())
        }

        pub fn switch_to_english(&self) -> Result<(), TisError> {
            Ok(())
        }
    }

    impl Default for InputSourceManager {
        fn default() -> Self {
            Self::new()
        }
    }
}
