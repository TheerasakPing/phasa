//! Text Input Source (TIS) manager for querying and switching macOS keyboard layouts.

use crate::ffi;
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use std::sync::Mutex;
use thiserror::Error;

/// Global mutex to serialize Carbon TIS API calls, which are not thread-safe.
static TIS_LOCK: Mutex<()> = Mutex::new(());

/// Information about an installed or enabled keyboard input source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSourceInfo {
    /// Bundle identifier of the input source (e.g. "com.apple.keylayout.ABC")
    pub id: String,
    /// Localized human-readable name (e.g. "ABC", "Thai")
    pub name: String,
    /// List of ISO language tags associated with this source (e.g. ["en"], ["th"])
    pub languages: Vec<String>,
}

#[derive(Debug, Error)]
pub enum TisError {
    #[error("Failed to copy current input source")]
    CannotGetCurrentSource,
    #[error("Failed to list keyboard input sources")]
    CannotListSources,
    #[error("Input source '{0}' not found")]
    SourceNotFound(String),
    #[error("Failed to select input source '{source_id}': OSStatus {status}")]
    SelectFailed { source_id: String, status: i32 },
    #[error("Property '{0}' not found on input source")]
    PropertyNotFound(String),
}

struct Invocation<F, R> {
    closure: Option<F>,
    result: Option<R>,
}

unsafe extern "C" fn dispatch_sync_runner<F: FnOnce() -> R, R>(context: *mut libc::c_void) {
    let inv = &mut *(context as *mut Invocation<F, R>);
    if let Some(f) = inv.closure.take() {
        inv.result = Some(f());
    }
}

/// Checks whether an AppKit `NSApplication` event loop is actively running.
pub(crate) fn is_appkit_running() -> bool {
    unsafe {
        let class_name = b"NSApplication\0".as_ptr() as *const libc::c_char;
        let nsapp_class = ffi::objc_getClass(class_name);
        if nsapp_class.is_null() {
            return false;
        }

        let sel_shared =
            ffi::sel_registerName(b"sharedApplication\0".as_ptr() as *const libc::c_char);
        let shared_app: *mut libc::c_void = {
            let msg_send: unsafe extern "C" fn(
                *mut libc::c_void,
                *mut libc::c_void,
            ) -> *mut libc::c_void = std::mem::transmute(ffi::objc_msgSend as *const ());
            msg_send(nsapp_class, sel_shared)
        };
        if shared_app.is_null() {
            return false;
        }

        let sel_running = ffi::sel_registerName(b"isRunning\0".as_ptr() as *const libc::c_char);
        let is_running: libc::c_uchar = {
            let msg_send: unsafe extern "C" fn(
                *mut libc::c_void,
                *mut libc::c_void,
            ) -> libc::c_uchar = std::mem::transmute(ffi::objc_msgSend as *const ());
            msg_send(shared_app, sel_running)
        };

        is_running != 0
    }
}

/// Runs closure on the main dispatch queue if an AppKit event loop is active,
/// or executes immediately on the current thread if already on the main thread or
/// in headless/test mode.
pub(crate) fn run_on_main<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    unsafe {
        if libc::pthread_main_np() != 0 || !is_appkit_running() {
            return f();
        }

        let mut inv = Invocation {
            closure: Some(f),
            result: None,
        };

        let context = &mut inv as *mut Invocation<F, R> as *mut libc::c_void;
        let queue_ptr = std::ptr::addr_of!(ffi::_dispatch_main_q);

        ffi::dispatch_sync_f(queue_ptr, context, dispatch_sync_runner::<F, R>);

        inv.result.expect("run_on_main: closure failed to return result")
    }
}

/// Manager for macOS Text Input Sources (Carbon TIS API).
#[derive(Debug, Default, Clone)]
pub struct InputSourceManager;

impl InputSourceManager {
    /// Creates a new `InputSourceManager`.
    pub fn new() -> Self {
        Self
    }

    /// Returns the input source ID of the currently active keyboard input source.
    pub fn get_current_source_id(&self) -> Result<String, TisError> {
        run_on_main(|| {
            let _guard = TIS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            self.get_current_source_id_unlocked()
        })
    }

    fn get_current_source_id_unlocked(&self) -> Result<String, TisError> {
        unsafe {
            let mut source = ffi::TISCopyCurrentKeyboardInputSource();
            if source.is_null() {
                source = ffi::TISCopyCurrentKeyboardLayoutInputSource();
            }
            if source.is_null() {
                return Err(TisError::CannotGetCurrentSource);
            }

            let id_prop = ffi::TISGetInputSourceProperty(source, ffi::kTISPropertyInputSourceID);
            let id = if !id_prop.is_null() {
                cfstring_to_string(id_prop as ffi::CFStringRef)
            } else {
                None
            };

            ffi::CFRelease(source as ffi::CFTypeRef);

            id.ok_or_else(|| TisError::PropertyNotFound("kTISPropertyInputSourceID".to_string()))
        }
    }

    /// Lists all enabled keyboard input sources.
    pub fn list_keyboard_input_sources(&self) -> Result<Vec<InputSourceInfo>, TisError> {
        run_on_main(|| {
            let _guard = TIS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            self.list_input_sources_unlocked(false)
        })
    }

    /// Lists all installed keyboard input sources (including disabled).
    pub fn list_all_installed_keyboard_input_sources(
        &self,
    ) -> Result<Vec<InputSourceInfo>, TisError> {
        run_on_main(|| {
            let _guard = TIS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            self.list_input_sources_unlocked(true)
        })
    }

    fn list_input_sources_unlocked(
        &self,
        include_all: bool,
    ) -> Result<Vec<InputSourceInfo>, TisError> {
        unsafe {
            let list =
                ffi::TISCreateInputSourceList(std::ptr::null(), if include_all { 1 } else { 0 });
            if list.is_null() {
                return Err(TisError::CannotListSources);
            }

            let count = ffi::CFArrayGetCount(list);
            let mut sources = Vec::new();

            for i in 0..count {
                let item = ffi::CFArrayGetValueAtIndex(list, i) as ffi::TISInputSourceRef;
                if is_keyboard_input_source(item) {
                    if let Some(info) = extract_source_info(item) {
                        sources.push(info);
                    }
                }
            }

            ffi::CFRelease(list as ffi::CFTypeRef);
            Ok(sources)
        }
    }

    /// Selects an input source by its bundle ID (e.g. "com.apple.keylayout.Thai").
    pub fn select_input_source(&self, source_id: &str) -> Result<(), TisError> {
        let source_id = source_id.to_string();
        run_on_main(move || {
            let _guard = TIS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            self.select_input_source_unlocked(&source_id)
        })
    }

    fn select_input_source_unlocked(&self, source_id: &str) -> Result<(), TisError> {
        unsafe {
            // First search enabled sources
            let mut list = ffi::TISCreateInputSourceList(std::ptr::null(), 0);
            let mut target_source: Option<ffi::TISInputSourceRef> = None;

            if !list.is_null() {
                let count = ffi::CFArrayGetCount(list);
                for i in 0..count {
                    let item = ffi::CFArrayGetValueAtIndex(list, i) as ffi::TISInputSourceRef;
                    let id_prop =
                        ffi::TISGetInputSourceProperty(item, ffi::kTISPropertyInputSourceID);
                    if !id_prop.is_null() {
                        if let Some(id_str) = cfstring_to_string(id_prop as ffi::CFStringRef) {
                            if id_str == source_id {
                                target_source = Some(item);
                                break;
                            }
                        }
                    }
                }
            }

            // If not found in enabled sources, search all installed sources
            if target_source.is_none() {
                if !list.is_null() {
                    ffi::CFRelease(list as ffi::CFTypeRef);
                }
                list = ffi::TISCreateInputSourceList(std::ptr::null(), 1);
                if !list.is_null() {
                    let count = ffi::CFArrayGetCount(list);
                    for i in 0..count {
                        let item = ffi::CFArrayGetValueAtIndex(list, i) as ffi::TISInputSourceRef;
                        let id_prop =
                            ffi::TISGetInputSourceProperty(item, ffi::kTISPropertyInputSourceID);
                        if !id_prop.is_null() {
                            if let Some(id_str) = cfstring_to_string(id_prop as ffi::CFStringRef) {
                                if id_str == source_id {
                                    target_source = Some(item);
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            let status = if let Some(source_ref) = target_source {
                ffi::TISSelectInputSource(source_ref)
            } else {
                if !list.is_null() {
                    ffi::CFRelease(list as ffi::CFTypeRef);
                }
                return Err(TisError::SourceNotFound(source_id.to_string()));
            };

            if !list.is_null() {
                ffi::CFRelease(list as ffi::CFTypeRef);
            }

            if status != ffi::NO_ERR {
                return Err(TisError::SelectFailed {
                    source_id: source_id.to_string(),
                    status,
                });
            }

            Ok(())
        }
    }

    /// Automatically finds and switches to an active Thai keyboard layout.
    /// Returns `Ok(true)` if Thai layout is now active, or `Ok(false)` if no Thai layout was found.
    pub fn switch_to_thai(&self) -> Result<bool, TisError> {
        run_on_main(|| {
            let _guard = TIS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let sources = self.list_input_sources_unlocked(false)?;
            let thai_source = sources.iter().find(|s| {
                s.languages.iter().any(|l| l.starts_with("th"))
                    || s.id.to_lowercase().contains("thai")
                    || s.name.to_lowercase().contains("thai")
            });

            if let Some(target) = thai_source {
                let current_id = self.get_current_source_id_unlocked().ok();
                if current_id.as_deref() == Some(&target.id) {
                    return Ok(true);
                }
                self.select_input_source_unlocked(&target.id)?;
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }

    /// Automatically finds and switches to an active English keyboard layout (e.g. ABC, US).
    /// Returns `Ok(true)` if English layout is now active, or `Ok(false)` if no English layout was found.
    pub fn switch_to_english(&self) -> Result<bool, TisError> {
        run_on_main(|| {
            let _guard = TIS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let sources = self.list_input_sources_unlocked(false)?;
            let en_source = sources.iter().find(|s| {
                s.languages
                    .first()
                    .map(|l| l.starts_with("en"))
                    .unwrap_or(false)
                    || s.id.contains("ABC")
                    || s.id.contains("US")
                    || s.name.to_lowercase().contains("english")
            });

            if let Some(target) = en_source {
                let current_id = self.get_current_source_id_unlocked().ok();
                if current_id.as_deref() == Some(&target.id) {
                    return Ok(true);
                }
                self.select_input_source_unlocked(&target.id)?;
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }
}

/// Converts a `CFStringRef` to an owned `String`.
pub(crate) unsafe fn cfstring_to_string(cf_str: ffi::CFStringRef) -> Option<String> {
    if cf_str.is_null() {
        return None;
    }
    let s = CFString::wrap_under_get_rule(cf_str);
    Some(s.to_string())
}

/// Converts a `CFArrayRef` of `CFStringRef` to a `Vec<String>`.
pub(crate) unsafe fn cfarray_to_string_vec(cf_arr: ffi::CFArrayRef) -> Vec<String> {
    if cf_arr.is_null() {
        return Vec::new();
    }
    let count = ffi::CFArrayGetCount(cf_arr);
    let mut vec = Vec::with_capacity(count as usize);
    for i in 0..count {
        let item = ffi::CFArrayGetValueAtIndex(cf_arr, i) as ffi::CFStringRef;
        if let Some(s) = cfstring_to_string(item) {
            vec.push(s);
        }
    }
    vec
}

/// Determines whether a `TISInputSourceRef` represents a keyboard input source.
unsafe fn is_keyboard_input_source(source: ffi::TISInputSourceRef) -> bool {
    let cat = ffi::TISGetInputSourceProperty(source, ffi::kTISPropertyInputSourceCategory);
    if cat.is_null() {
        return false;
    }
    if let Some(cat_str) = cfstring_to_string(cat as ffi::CFStringRef) {
        cat_str == "TISCategoryKeyboardInputSource"
    } else {
        false
    }
}

/// Extracts `InputSourceInfo` metadata from a `TISInputSourceRef`.
unsafe fn extract_source_info(source: ffi::TISInputSourceRef) -> Option<InputSourceInfo> {
    let id_prop = ffi::TISGetInputSourceProperty(source, ffi::kTISPropertyInputSourceID);
    let id = cfstring_to_string(id_prop as ffi::CFStringRef)?;

    let name_prop = ffi::TISGetInputSourceProperty(source, ffi::kTISPropertyLocalizedName);
    let name = cfstring_to_string(name_prop as ffi::CFStringRef).unwrap_or_else(|| id.clone());

    let langs_prop = ffi::TISGetInputSourceProperty(source, ffi::kTISPropertyInputSourceLanguages);
    let languages = if !langs_prop.is_null() {
        cfarray_to_string_vec(langs_prop as ffi::CFArrayRef)
    } else {
        Vec::new()
    };

    Some(InputSourceInfo {
        id,
        name,
        languages,
    })
}
