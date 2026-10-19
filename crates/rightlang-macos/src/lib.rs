//! Platform hook and dispatcher for macOS.

#[cfg(target_os = "macos")]
pub mod clipboard;
#[cfg(target_os = "macos")]
pub mod dispatcher;
#[cfg(target_os = "macos")]
pub mod event;
#[cfg(target_os = "macos")]
pub mod ffi;
#[cfg(target_os = "macos")]
pub mod hook;
#[cfg(target_os = "macos")]
pub mod tis;

#[cfg(target_os = "macos")]
pub use clipboard::{get_clipboard_text, set_clipboard_text};
#[cfg(target_os = "macos")]
pub use dispatcher::{DispatchError, KeyDispatcher};
#[cfg(target_os = "macos")]
pub use event::{
    extract_characters, is_event_tagged_as_rightlang, is_rightlang_event, tag_event_as_rightlang,
    KeyEvent, KeyEventType, RIGHTLANG_EVENT_TAG,
};
#[cfg(target_os = "macos")]
pub use hook::{is_accessibility_trusted, EventTapListener, HookError};
#[cfg(target_os = "macos")]
pub use tis::{InputSourceInfo, InputSourceManager, TisError};

#[cfg(not(target_os = "macos"))]
pub mod non_macos;

#[cfg(not(target_os = "macos"))]
pub use non_macos::{clipboard, dispatcher, event, ffi, hook, tis};

#[cfg(not(target_os = "macos"))]
pub use non_macos::clipboard::{get_clipboard_text, set_clipboard_text};
#[cfg(not(target_os = "macos"))]
pub use non_macos::dispatcher::{DispatchError, KeyDispatcher};
#[cfg(not(target_os = "macos"))]
pub use non_macos::event::{
    extract_characters, is_event_tagged_as_rightlang, is_rightlang_event, tag_event_as_rightlang,
    KeyEvent, KeyEventType, RIGHTLANG_EVENT_TAG,
};
#[cfg(not(target_os = "macos"))]
pub use non_macos::hook::{is_accessibility_trusted, EventTapListener, HookError};
#[cfg(not(target_os = "macos"))]
pub use non_macos::tis::{InputSourceInfo, InputSourceManager, TisError};
