//! Keyboard event representations and event tagging.

use serde::{Deserialize, Serialize};

/// Magic user data tag for RightLang-RS events to prevent infinite loops.
/// ASCII representation of "RLG0" = 0x524C4730.
pub const RIGHTLANG_EVENT_TAG: i64 = 0x524C4730;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyEventType {
    KeyDown,
    KeyUp,
    FlagsChanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEvent {
    pub key_code: u16,
    pub event_type: KeyEventType,
    pub flags: u64,
    pub characters: Option<String>,
    pub is_synthetic: bool,
}

impl KeyEvent {
    pub fn new(
        key_code: u16,
        event_type: KeyEventType,
        flags: u64,
        characters: Option<String>,
        is_synthetic: bool,
    ) -> Self {
        Self {
            key_code,
            event_type,
            flags,
            characters,
            is_synthetic,
        }
    }
}

/// Checks if a user data field value matches the RightLang event tag.
#[inline]
pub fn is_rightlang_event(user_data: i64) -> bool {
    user_data == RIGHTLANG_EVENT_TAG
}

/// Checks if a low-level CGEventRef is tagged with RightLang's magic user data tag.
///
/// # Safety
/// `event` must be a valid `CGEventRef` or null.
pub unsafe fn is_event_tagged_as_rightlang(event: crate::ffi::CGEventRef) -> bool {
    if event.is_null() {
        return false;
    }
    let user_data =
        crate::ffi::CGEventGetIntegerValueField(event, crate::ffi::K_CG_EVENT_SOURCE_USER_DATA);
    is_rightlang_event(user_data)
}

/// Tags a low-level CGEventRef with RightLang's magic user data tag.
///
/// # Safety
/// `event` must be a valid, mutable `CGEventRef` or null.
pub unsafe fn tag_event_as_rightlang(event: crate::ffi::CGEventRef) {
    if !event.is_null() {
        crate::ffi::CGEventSetIntegerValueField(
            event,
            crate::ffi::K_CG_EVENT_SOURCE_USER_DATA,
            RIGHTLANG_EVENT_TAG,
        );
    }
}

/// Extracts the Unicode characters string from a low-level CGEventRef.
///
/// # Safety
/// `event` must be a valid `CGEventRef` or null.
pub unsafe fn extract_characters(event: crate::ffi::CGEventRef) -> Option<String> {
    if event.is_null() {
        return None;
    }
    let mut actual_len: crate::ffi::UniCharCount = 0;
    let mut buffer = [0u16; 16];
    crate::ffi::CGEventKeyboardGetUnicodeString(
        event,
        buffer.len() as crate::ffi::UniCharCount,
        &mut actual_len,
        buffer.as_mut_ptr(),
    );

    if actual_len == 0 {
        return None;
    }

    if (actual_len as usize) <= buffer.len() {
        return String::from_utf16(&buffer[..actual_len as usize]).ok();
    }

    // Fallback for long characters buffer
    let mut long_buffer = vec![0u16; actual_len as usize];
    let mut second_len: crate::ffi::UniCharCount = 0;
    crate::ffi::CGEventKeyboardGetUnicodeString(
        event,
        long_buffer.len() as crate::ffi::UniCharCount,
        &mut second_len,
        long_buffer.as_mut_ptr(),
    );

    let len = (second_len as usize).min(long_buffer.len());
    String::from_utf16(&long_buffer[..len]).ok()
}
