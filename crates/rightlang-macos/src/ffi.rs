//! Low-level macOS FFI bindings for CoreGraphics, ApplicationServices, and Carbon.

use std::ffi::c_void;

pub use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex, CFArrayRef};
pub use core_foundation_sys::base::{Boolean, CFIndex, CFRelease, CFTypeRef};
pub use core_foundation_sys::dictionary::{
    kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, CFDictionaryCreate,
    CFDictionaryRef,
};
pub use core_foundation_sys::mach_port::{
    CFMachPortCreateRunLoopSource, CFMachPortInvalidate, CFMachPortRef,
};
pub use core_foundation_sys::number::{kCFBooleanTrue, CFBooleanRef};
pub use core_foundation_sys::runloop::{
    kCFRunLoopCommonModes, CFRunLoopAddSource, CFRunLoopGetCurrent, CFRunLoopRef,
    CFRunLoopRemoveSource, CFRunLoopRun, CFRunLoopSourceRef, CFRunLoopStop,
};
pub use core_foundation_sys::string::CFStringRef;

#[repr(C)]
pub struct __CGEvent(c_void);
pub type CGEventRef = *mut __CGEvent;

#[repr(C)]
pub struct __CGEventSource(c_void);
pub type CGEventSourceRef = *mut __CGEventSource;

pub type CGEventTapProxy = *mut c_void;

pub type CGEventTapCallBack = unsafe extern "C" fn(
    proxy: CGEventTapProxy,
    etype: u32,
    event: CGEventRef,
    refcon: *mut c_void,
) -> CGEventRef;

pub type CGEventMask = u64;
pub type CGKeyCode = u16;
pub type CGEventFlags = u64;
pub type UniCharCount = libc::c_ulong;
pub type UniChar = u16;

// Event tap locations
pub const K_CG_HID_EVENT_TAP: u32 = 0;
pub const K_CG_SESSION_EVENT_TAP: u32 = 1;
pub const K_CG_ANNOTATED_SESSION_EVENT_TAP: u32 = 2;

// Event tap placements
pub const K_CG_HEAD_INSERT_EVENT_TAP: u32 = 0;
pub const K_CG_TAIL_APPEND_EVENT_TAP: u32 = 1;

// Event tap options
pub const K_CG_EVENT_TAP_OPTION_DEFAULT: u32 = 0;
pub const K_CG_EVENT_TAP_OPTION_LISTEN_ONLY: u32 = 1;

// CGEventField keys
pub const K_CG_KEYBOARD_EVENT_AUTOREPEAT: u32 = 8;
pub const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;
pub const K_CG_KEYBOARD_EVENT_KEYBOARD_TYPE: u32 = 10;
pub const K_CG_EVENT_SOURCE_USER_DATA: u32 = 42;
pub const K_CG_EVENT_SOURCE_STATE_ID: u32 = 45;

// CGEventType values
pub const K_CG_EVENT_KEY_DOWN: u32 = 10;
pub const K_CG_EVENT_KEY_UP: u32 = 11;
pub const K_CG_EVENT_FLAGS_CHANGED: u32 = 12;

// Out-of-band tap disable events
pub const K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT: u32 = 0xFFFFFFFE;
pub const K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT: u32 = 0xFFFFFFFF;

// KeyCode constants
pub const KEY_CODE_C: CGKeyCode = 0x08; // 8
pub const KEY_CODE_V: CGKeyCode = 0x09; // 9
pub const KEY_CODE_SLASH: CGKeyCode = 0x2C; // 44
pub const KEY_CODE_SPACE: CGKeyCode = 0x31; // 49
pub const KEY_CODE_BACKSPACE: CGKeyCode = 0x33; // 51

// CGEventFlags modifier masks
pub const K_CG_EVENT_FLAG_MASK_SHIFT: CGEventFlags = 0x00020000;
pub const K_CG_EVENT_FLAG_MASK_CONTROL: CGEventFlags = 0x00040000;
pub const K_CG_EVENT_FLAG_MASK_ALTERNATE: CGEventFlags = 0x00080000;
pub const K_CG_EVENT_FLAG_MASK_COMMAND: CGEventFlags = 0x00100000;

// Carbon Text Input Source (TIS) types and status
#[repr(C)]
pub struct __TISInputSource(c_void);
pub type TISInputSourceRef = *mut __TISInputSource;
pub type OSStatus = i32;
pub const NO_ERR: OSStatus = 0;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: CGEventMask,
        callback: CGEventTapCallBack,
        refcon: *mut c_void,
    ) -> CFMachPortRef;

    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);

    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    pub fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);

    pub fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
    pub fn CGEventSetFlags(event: CGEventRef, flags: CGEventFlags);

    pub fn CGEventKeyboardGetUnicodeString(
        event: CGEventRef,
        max_string_length: UniCharCount,
        actual_string_length: *mut UniCharCount,
        unicode_string: *mut UniChar,
    );

    pub fn CGEventKeyboardSetUnicodeString(
        event: CGEventRef,
        string_length: UniCharCount,
        unicode_string: *const UniChar,
    );

    pub fn CGEventCreateKeyboardEvent(
        source: CGEventSourceRef,
        virtual_key: CGKeyCode,
        key_down: bool,
    ) -> CGEventRef;

    pub fn CGEventPost(tap: u32, event: CGEventRef);
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub static kAXTrustedCheckOptionPrompt: CFStringRef;
    pub fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> Boolean;
}

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub static kTISPropertyInputSourceID: CFStringRef;
    pub static kTISPropertyLocalizedName: CFStringRef;
    pub static kTISPropertyInputSourceCategory: CFStringRef;
    pub static kTISPropertyInputSourceLanguages: CFStringRef;
    pub static kTISPropertyInputSourceIsSelectCapable: CFStringRef;
    pub static kTISPropertyInputSourceIsEnabled: CFStringRef;

    pub static kTISCategoryKeyboardInputSource: CFStringRef;

    pub fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
    pub fn TISCopyCurrentKeyboardLayoutInputSource() -> TISInputSourceRef;
    pub fn TISCreateInputSourceList(
        properties: CFDictionaryRef,
        includeAllInstalled: Boolean,
    ) -> CFArrayRef;
    pub fn TISGetInputSourceProperty(
        source: TISInputSourceRef,
        propertyKey: CFStringRef,
    ) -> *const c_void;
    pub fn TISSelectInputSource(source: TISInputSourceRef) -> OSStatus;
}

#[link(name = "System", kind = "dylib")]
extern "C" {
    pub static _dispatch_main_q: libc::c_void;
    pub fn dispatch_sync_f(
        queue: *const libc::c_void,
        context: *mut libc::c_void,
        work: unsafe extern "C" fn(*mut libc::c_void),
    );
}

#[link(name = "objc")]
extern "C" {
    pub fn objc_getClass(name: *const libc::c_char) -> *mut libc::c_void;
    pub fn sel_registerName(name: *const libc::c_char) -> *mut libc::c_void;
    pub fn objc_msgSend();
}
