//! Platform hook and dispatcher for macOS.

pub mod dispatcher;
pub mod event;
pub mod ffi;
pub mod hook;
pub mod tis;

pub use dispatcher::{DispatchError, KeyDispatcher};
pub use event::{
    extract_characters, is_event_tagged_as_rightlang, is_rightlang_event, tag_event_as_rightlang,
    KeyEvent, KeyEventType, RIGHTLANG_EVENT_TAG,
};
pub use hook::{is_accessibility_trusted, EventTapListener, HookError};
pub use tis::{InputSourceInfo, InputSourceManager, TisError};
