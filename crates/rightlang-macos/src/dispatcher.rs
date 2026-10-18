//! Synthetic keyboard event dispatcher using macOS CoreGraphics CGEvents.

use std::time::Duration;
use thiserror::Error;

use crate::event::tag_event_as_rightlang;
use crate::ffi::{self, CGEventRef, UniCharCount, KEY_CODE_BACKSPACE, K_CG_HID_EVENT_TAP};

#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("Failed to allocate CGEvent")]
    EventCreationFailed,
}

/// Dispatches synthetic keyboard events with microsecond timing.
#[derive(Debug, Clone)]
pub struct KeyDispatcher {
    default_delay_us: u64,
}

impl KeyDispatcher {
    /// Creates a new `KeyDispatcher` with the specified default inter-key delay in microseconds.
    pub fn new(default_delay_us: u64) -> Self {
        Self { default_delay_us }
    }

    /// Returns the default inter-key delay in microseconds.
    pub fn default_delay_us(&self) -> u64 {
        self.default_delay_us
    }

    /// Helper to create a tagged synthetic Backspace keyboard event.
    ///
    /// # Safety
    /// Caller owns the returned `CGEventRef` and must release it with `CFRelease`.
    pub fn create_backspace_event(key_down: bool) -> Result<CGEventRef, DispatchError> {
        unsafe {
            let event =
                ffi::CGEventCreateKeyboardEvent(std::ptr::null_mut(), KEY_CODE_BACKSPACE, key_down);
            if event.is_null() {
                return Err(DispatchError::EventCreationFailed);
            }
            tag_event_as_rightlang(event);
            Ok(event)
        }
    }

    /// Helper to create a tagged synthetic Unicode keyboard event.
    ///
    /// # Safety
    /// Caller owns the returned `CGEventRef` and must release it with `CFRelease`.
    pub fn create_unicode_event(
        utf16_chars: &[u16],
        key_down: bool,
    ) -> Result<CGEventRef, DispatchError> {
        unsafe {
            let event = ffi::CGEventCreateKeyboardEvent(std::ptr::null_mut(), 0, key_down);
            if event.is_null() {
                return Err(DispatchError::EventCreationFailed);
            }
            tag_event_as_rightlang(event);
            ffi::CGEventKeyboardSetUnicodeString(
                event,
                utf16_chars.len() as UniCharCount,
                utf16_chars.as_ptr(),
            );
            Ok(event)
        }
    }

    /// Dispatches `count` synthetic Backspace keystrokes, with `delay_us` delay between each.
    pub fn send_backspaces(&self, count: usize, delay_us: u64) -> Result<(), DispatchError> {
        if count == 0 {
            return Ok(());
        }

        for _ in 0..count {
            unsafe {
                let event_down = Self::create_backspace_event(true)?;
                ffi::CGEventPost(K_CG_HID_EVENT_TAP, event_down);
                ffi::CFRelease(event_down as ffi::CFTypeRef);

                let event_up = Self::create_backspace_event(false)?;
                ffi::CGEventPost(K_CG_HID_EVENT_TAP, event_up);
                ffi::CFRelease(event_up as ffi::CFTypeRef);
            }

            if delay_us > 0 {
                std::thread::sleep(Duration::from_micros(delay_us));
            }
        }

        Ok(())
    }

    /// Dispatches a Unicode string as keyboard events (handling UTF-16 surrogate pairs and combining characters).
    pub fn send_text(&self, text: &str, delay_us: u64) -> Result<(), DispatchError> {
        if text.is_empty() {
            return Ok(());
        }

        for ch in text.chars() {
            let mut buf = [0u16; 2];
            let utf16_slice = ch.encode_utf16(&mut buf);

            unsafe {
                let event_down = Self::create_unicode_event(utf16_slice, true)?;
                ffi::CGEventPost(K_CG_HID_EVENT_TAP, event_down);
                ffi::CFRelease(event_down as ffi::CFTypeRef);

                let event_up = Self::create_unicode_event(utf16_slice, false)?;
                ffi::CGEventPost(K_CG_HID_EVENT_TAP, event_up);
                ffi::CFRelease(event_up as ffi::CFTypeRef);
            }

            if delay_us > 0 {
                std::thread::sleep(Duration::from_micros(delay_us));
            }
        }

        Ok(())
    }

    /// Dispatches a synthetic key combination (key down followed by key up with modifier flags).
    ///
    /// Every event is tagged with `RIGHTLANG_EVENT_TAG` so `EventTapListener` ignores them.
    pub fn send_key_combination(
        &self,
        keycode: u16,
        flags: u64,
        delay_us: u64,
    ) -> Result<(), DispatchError> {
        unsafe {
            let event_down = ffi::CGEventCreateKeyboardEvent(std::ptr::null_mut(), keycode, true);
            if event_down.is_null() {
                return Err(DispatchError::EventCreationFailed);
            }
            tag_event_as_rightlang(event_down);
            ffi::CGEventSetFlags(event_down, flags);
            ffi::CGEventPost(K_CG_HID_EVENT_TAP, event_down);
            ffi::CFRelease(event_down as ffi::CFTypeRef);

            if delay_us > 0 {
                std::thread::sleep(Duration::from_micros(delay_us));
            }

            let event_up = ffi::CGEventCreateKeyboardEvent(std::ptr::null_mut(), keycode, false);
            if event_up.is_null() {
                return Err(DispatchError::EventCreationFailed);
            }
            tag_event_as_rightlang(event_up);
            ffi::CGEventSetFlags(event_up, flags);
            ffi::CGEventPost(K_CG_HID_EVENT_TAP, event_up);
            ffi::CFRelease(event_up as ffi::CFTypeRef);

            if delay_us > 0 {
                std::thread::sleep(Duration::from_micros(delay_us));
            }
        }

        Ok(())
    }

    /// Dispatches `backspace_count` synthetic backspaces, followed by typing `replacement` text.
    ///
    /// Every event is tagged with `RIGHTLANG_EVENT_TAG` so `EventTapListener` ignores them.
    pub fn replace_text(
        &self,
        backspace_count: usize,
        replacement: &str,
        delay_us: Option<u64>,
    ) -> Result<(), DispatchError> {
        let delay = delay_us.unwrap_or(self.default_delay_us);
        self.send_backspaces(backspace_count, delay)?;
        self.send_text(replacement, delay)?;
        Ok(())
    }
}

impl Default for KeyDispatcher {
    fn default() -> Self {
        Self::new(2000)
    }
}
