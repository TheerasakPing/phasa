//! Per-application correction rules and frontmost application detection.

use std::collections::HashMap;

use crate::config::{AppConfig, CorrectionMode};

/// Manages per-application correction rules based on application bundle identifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationRuleManager {
    rules: HashMap<String, CorrectionMode>,
    default_mode: CorrectionMode,
}

impl ApplicationRuleManager {
    /// Creates a new rule manager with the given default mode and app rule map.
    pub fn new(default_mode: CorrectionMode, rules: HashMap<String, CorrectionMode>) -> Self {
        Self {
            rules,
            default_mode,
        }
    }

    /// Creates an application rule manager from an `AppConfig`.
    pub fn from_config(config: &AppConfig) -> Self {
        Self::new(config.mode, config.app_rules.clone())
    }

    /// Returns the global default mode.
    pub fn default_mode(&self) -> CorrectionMode {
        self.default_mode
    }

    /// Sets the global default mode.
    pub fn set_default_mode(&mut self, mode: CorrectionMode) {
        self.default_mode = mode;
    }

    /// Returns a reference to all application rules.
    pub fn rules(&self) -> &HashMap<String, CorrectionMode> {
        &self.rules
    }

    /// Returns the effective correction mode for a specific application bundle identifier.
    /// If no explicit rule matches the bundle ID, the default mode is returned.
    pub fn mode_for_app(&self, bundle_id: &str) -> CorrectionMode {
        let trimmed = bundle_id.trim();
        self.rules
            .get(bundle_id)
            .or_else(|| self.rules.get(trimmed))
            .copied()
            .unwrap_or(self.default_mode)
    }

    /// Sets or updates a correction rule for an application bundle identifier.
    pub fn set_rule(&mut self, bundle_id: &str, mode: CorrectionMode) {
        self.rules.insert(bundle_id.trim().to_string(), mode);
    }

    /// Removes an application rule, causing the app to fall back to the default mode.
    pub fn remove_rule(&mut self, bundle_id: &str) {
        let trimmed = bundle_id.trim();
        self.rules.remove(bundle_id);
        if trimmed != bundle_id {
            self.rules.remove(trimmed);
        }
    }

    /// Returns the correction mode for the currently frontmost active application.
    /// If the active application cannot be determined, returns the default mode.
    pub fn current_app_mode(&self) -> CorrectionMode {
        match get_frontmost_app_bundle_id() {
            Some(bundle_id) => self.mode_for_app(&bundle_id),
            None => self.default_mode,
        }
    }
}

/// Detects the frontmost active application's bundle identifier on macOS.
///
/// Uses `NSWorkspace.sharedWorkspace.frontmostApplication.bundleIdentifier` via
/// Objective-C runtime FFI, falling back to `lsappinfo` CLI if necessary.
#[cfg(target_os = "macos")]
pub fn get_frontmost_app_bundle_id() -> Option<String> {
    get_frontmost_app_bundle_id_objc().or_else(get_frontmost_app_bundle_id_cli)
}

/// Fallback for non-macOS platforms.
#[cfg(not(target_os = "macos"))]
pub fn get_frontmost_app_bundle_id() -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
fn get_frontmost_app_bundle_id_objc() -> Option<String> {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    #[link(name = "AppKit", kind = "framework")]
    #[link(name = "objc", kind = "dylib")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut std::ffi::c_void;
        fn sel_registerName(name: *const c_char) -> *mut std::ffi::c_void;
        fn objc_msgSend();
    }

    type MsgSendNoArgs = unsafe extern "C" fn(
        *mut std::ffi::c_void,
        *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    type MsgSendUtf8 =
        unsafe extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *const c_char;

    unsafe {
        let msg_send: MsgSendNoArgs = std::mem::transmute(objc_msgSend as *const ());
        let msg_send_utf8: MsgSendUtf8 = std::mem::transmute(objc_msgSend as *const ());

        let cls_nsworkspace = objc_getClass(c"NSWorkspace".as_ptr());
        if cls_nsworkspace.is_null() {
            return None;
        }

        let sel_shared = sel_registerName(c"sharedWorkspace".as_ptr());
        let workspace = msg_send(cls_nsworkspace, sel_shared);
        if workspace.is_null() {
            return None;
        }

        let sel_frontmost = sel_registerName(c"frontmostApplication".as_ptr());
        let app = msg_send(workspace, sel_frontmost);
        if app.is_null() {
            return None;
        }

        let sel_bundle_id = sel_registerName(c"bundleIdentifier".as_ptr());
        let bundle_id_nsstring = msg_send(app, sel_bundle_id);
        if bundle_id_nsstring.is_null() {
            return None;
        }

        let sel_utf8 = sel_registerName(c"UTF8String".as_ptr());
        let utf8_ptr = msg_send_utf8(bundle_id_nsstring, sel_utf8);
        if utf8_ptr.is_null() {
            return None;
        }

        let cstr = CStr::from_ptr(utf8_ptr);
        let bundle_str = cstr.to_str().ok()?;
        if bundle_str.is_empty() {
            None
        } else {
            Some(bundle_str.to_string())
        }
    }
}

#[cfg(target_os = "macos")]
fn get_frontmost_app_bundle_id_cli() -> Option<String> {
    use std::process::Command;

    let front_out = Command::new("lsappinfo").arg("front").output().ok()?;
    if !front_out.status.success() {
        return None;
    }

    let asn = String::from_utf8_lossy(&front_out.stdout).trim().to_string();
    if asn.is_empty() {
        return None;
    }

    let info_out = Command::new("lsappinfo")
        .args(["info", "-bundleid", &asn])
        .output()
        .ok()?;
    if !info_out.status.success() {
        return None;
    }

    let info_str = String::from_utf8_lossy(&info_out.stdout);
    for line in info_str.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("bundleID=\"") {
            if let Some(id) = rest.strip_suffix('"') {
                if !id.is_empty() {
                    return Some(id.to_string());
                }
            }
        }
    }

    None
}
