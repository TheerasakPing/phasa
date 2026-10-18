//! Safe native macOS clipboard (NSPasteboard) utilities using AppKit FFI.

use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::Mutex;

static PASTEBOARD_LOCK: Mutex<()> = Mutex::new(());

#[link(name = "AppKit", kind = "framework")]
#[link(name = "objc", kind = "dylib")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
    static NSPasteboardTypeString: *mut c_void;
}

/// Retrieves plain text content currently on the macOS general pasteboard (`NSPasteboard`).
///
/// Returns `Some(String)` containing the UTF-8 text if available, or `None` if empty
/// or if the pasteboard does not contain plain text.
pub fn get_clipboard_text() -> Option<String> {
    let _guard = PASTEBOARD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let result = get_clipboard_text_inner();
        objc_autoreleasePoolPop(pool);
        result
    }
}

unsafe fn get_clipboard_text_inner() -> Option<String> {
    let cls_pasteboard = objc_getClass(c"NSPasteboard".as_ptr());
    if cls_pasteboard.is_null() {
        return None;
    }

    type MsgSendNoArgs = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type MsgSendOneArg = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void;
    type MsgSendToCStr = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const c_char;

    let msg_send_no_args: MsgSendNoArgs = std::mem::transmute(objc_msgSend as *const ());
    let msg_send_one_arg: MsgSendOneArg = std::mem::transmute(objc_msgSend as *const ());
    let msg_send_to_cstr: MsgSendToCStr = std::mem::transmute(objc_msgSend as *const ());

    let sel_general = sel_registerName(c"generalPasteboard".as_ptr());
    let pasteboard = msg_send_no_args(cls_pasteboard, sel_general);
    if pasteboard.is_null() {
        return None;
    }

    let type_str = if !NSPasteboardTypeString.is_null() {
        NSPasteboardTypeString
    } else {
        let cls_nsstring = objc_getClass(c"NSString".as_ptr());
        let sel_str = sel_registerName(c"stringWithUTF8String:".as_ptr());
        type MsgSendCStr =
            unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
        let msg_send_cstr: MsgSendCStr = std::mem::transmute(objc_msgSend as *const ());
        msg_send_cstr(cls_nsstring, sel_str, c"public.utf8-plain-text".as_ptr())
    };

    let sel_string_for_type = sel_registerName(c"stringForType:".as_ptr());
    let ns_str = msg_send_one_arg(pasteboard, sel_string_for_type, type_str);
    if ns_str.is_null() {
        return None;
    }

    let sel_utf8 = sel_registerName(c"UTF8String".as_ptr());
    let c_str = msg_send_to_cstr(ns_str, sel_utf8);
    if c_str.is_null() {
        return None;
    }

    CStr::from_ptr(c_str).to_str().ok().map(|s| s.to_string())
}

/// Writes UTF-8 plain text onto the macOS general pasteboard (`NSPasteboard`).
///
/// Clears existing pasteboard contents and registers the string under plain text type.
/// Returns `true` if successfully written, or `false` on failure.
pub fn set_clipboard_text(text: &str) -> bool {
    let Ok(c_text) = CString::new(text) else {
        return false;
    };

    let _guard = PASTEBOARD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let result = set_clipboard_text_inner(&c_text);
        objc_autoreleasePoolPop(pool);
        result
    }
}

unsafe fn set_clipboard_text_inner(c_text: &CStr) -> bool {
    let cls_pasteboard = objc_getClass(c"NSPasteboard".as_ptr());
    let cls_nsstring = objc_getClass(c"NSString".as_ptr());
    if cls_pasteboard.is_null() || cls_nsstring.is_null() {
        return false;
    }

    type MsgSendNoArgs = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    type MsgSendClear = unsafe extern "C" fn(*mut c_void, *mut c_void) -> libc::intptr_t;
    type MsgSendCStr =
        unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
    type MsgSendSetString =
        unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> bool;

    let msg_send_no_args: MsgSendNoArgs = std::mem::transmute(objc_msgSend as *const ());
    let msg_send_clear: MsgSendClear = std::mem::transmute(objc_msgSend as *const ());
    let msg_send_cstr: MsgSendCStr = std::mem::transmute(objc_msgSend as *const ());
    let msg_send_set_string: MsgSendSetString = std::mem::transmute(objc_msgSend as *const ());

    let sel_general = sel_registerName(c"generalPasteboard".as_ptr());
    let pasteboard = msg_send_no_args(cls_pasteboard, sel_general);
    if pasteboard.is_null() {
        return false;
    }

    let sel_clear = sel_registerName(c"clearContents".as_ptr());
    msg_send_clear(pasteboard, sel_clear);

    let sel_str = sel_registerName(c"stringWithUTF8String:".as_ptr());
    let ns_string = msg_send_cstr(cls_nsstring, sel_str, c_text.as_ptr());
    if ns_string.is_null() {
        return false;
    }

    let type_str = if !NSPasteboardTypeString.is_null() {
        NSPasteboardTypeString
    } else {
        msg_send_cstr(cls_nsstring, sel_str, c"public.utf8-plain-text".as_ptr())
    };

    let sel_set_string = sel_registerName(c"setString:forType:".as_ptr());
    msg_send_set_string(pasteboard, sel_set_string, ns_string, type_str)
}
