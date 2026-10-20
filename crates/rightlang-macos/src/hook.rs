//! macOS CGEventTap keyboard hook listener and event filtering.

use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;

use crate::event::{self, KeyEvent, KeyEventType};
use crate::ffi;

/// Errors that can occur during event tap initialization and runloop execution.
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("Failed to create event tap: {0}")]
    CreationFailed(String),

    #[error("Invalid mach port")]
    InvalidPort,

    #[error("Run loop error: {0}")]
    RunLoopError(String),

    #[error("Listener already running")]
    AlreadyRunning,
}

/// Checks whether the current process is trusted for macOS Accessibility APIs.
/// If `prompt` is true, macOS prompts the user in System Settings if permission is missing.
pub fn is_accessibility_trusted(prompt: bool) -> bool {
    unsafe {
        if prompt {
            let key = ffi::kAXTrustedCheckOptionPrompt;
            let value = ffi::kCFBooleanTrue;
            let keys = [key as *const std::ffi::c_void];
            let values = [value as *const std::ffi::c_void];
            let dict = ffi::CFDictionaryCreate(
                std::ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                &ffi::kCFTypeDictionaryKeyCallBacks,
                &ffi::kCFTypeDictionaryValueCallBacks,
            );
            let trusted = ffi::AXIsProcessTrustedWithOptions(dict);
            if !dict.is_null() {
                ffi::CFRelease(dict as _);
            }
            trusted != 0
        } else {
            ffi::AXIsProcessTrustedWithOptions(std::ptr::null()) != 0
        }
    }
}

/// Shared context passed into the CGEventTap callback.
struct TapContext {
    sender: crossbeam_channel::Sender<KeyEvent>,
    mach_port: AtomicPtr<std::ffi::c_void>,
}

/// C-style event tap callback invoked by CoreGraphics.
unsafe extern "C" fn tap_callback(
    _proxy: ffi::CGEventTapProxy,
    etype: u32,
    event: ffi::CGEventRef,
    refcon: *mut std::ffi::c_void,
) -> ffi::CGEventRef {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if refcon.is_null() {
            return event;
        }
        let ctx = &*(refcon as *const TapContext);

        // Handle tap disabled by timeout or user input (automatic recovery)
        if etype == ffi::K_CG_EVENT_TAP_DISABLED_BY_TIMEOUT
            || etype == ffi::K_CG_EVENT_TAP_DISABLED_BY_USER_INPUT
        {
            let port = ctx.mach_port.load(Ordering::SeqCst);
            if !port.is_null() {
                ffi::CGEventTapEnable(port as _, true);
            }
            return event;
        }

        if event.is_null() {
            return event;
        }

        // Ignore our own synthetic events to prevent infinite loops
        let user_data = ffi::CGEventGetIntegerValueField(event, ffi::K_CG_EVENT_SOURCE_USER_DATA);
        if event::is_rightlang_event(user_data) {
            return event;
        }

        // Filter only relevant keyboard events
        let event_type = match etype {
            ffi::K_CG_EVENT_KEY_DOWN => KeyEventType::KeyDown,
            ffi::K_CG_EVENT_KEY_UP => KeyEventType::KeyUp,
            ffi::K_CG_EVENT_FLAGS_CHANGED => KeyEventType::FlagsChanged,
            _ => return event,
        };

        let key_code =
            ffi::CGEventGetIntegerValueField(event, ffi::K_CG_KEYBOARD_EVENT_KEYCODE) as u16;
        let flags = ffi::CGEventGetFlags(event);
        let characters = if event_type == KeyEventType::FlagsChanged {
            None
        } else {
            event::extract_characters(event)
        };

        // Determine if event is synthetic (non-hardware or has custom user data)
        let source_state = ffi::CGEventGetIntegerValueField(event, ffi::K_CG_EVENT_SOURCE_STATE_ID);
        let is_synthetic = source_state != 1 || user_data != 0;

        let key_event = KeyEvent {
            key_code,
            event_type,
            flags,
            characters,
            is_synthetic,
        };

        // Forward to the channel
        let _ = ctx.sender.send(key_event);

        event
    }));

    match result {
        Ok(ret) => ret,
        Err(_) => event,
    }
}

/// Robust event tap listener for low-level macOS keyboard events.
pub struct EventTapListener {
    mach_port: AtomicPtr<std::ffi::c_void>,
    run_loop: AtomicPtr<std::ffi::c_void>,
    is_running: Arc<AtomicBool>,
    _context: Arc<TapContext>,
}

impl EventTapListener {
    /// Creates a new `EventTapListener` targeting session keyboard events.
    pub fn new(sender: crossbeam_channel::Sender<KeyEvent>) -> Result<Self, HookError> {
        let context = Arc::new(TapContext {
            sender,
            mach_port: AtomicPtr::new(std::ptr::null_mut()),
        });

        let event_mask = (1u64 << ffi::K_CG_EVENT_KEY_DOWN)
            | (1u64 << ffi::K_CG_EVENT_KEY_UP)
            | (1u64 << ffi::K_CG_EVENT_FLAGS_CHANGED);

        let context_ptr = Arc::as_ptr(&context) as *mut std::ffi::c_void;

        let port = unsafe {
            ffi::CGEventTapCreate(
                ffi::K_CG_SESSION_EVENT_TAP,
                ffi::K_CG_HEAD_INSERT_EVENT_TAP,
                ffi::K_CG_EVENT_TAP_OPTION_DEFAULT,
                event_mask,
                tap_callback,
                context_ptr,
            )
        };

        if port.is_null() {
            return Err(HookError::CreationFailed(
                "CGEventTapCreate returned null. Verify Accessibility permissions under System Preferences -> Privacy & Security -> Accessibility.".into(),
            ));
        }

        context.mach_port.store(port as _, Ordering::SeqCst);

        Ok(Self {
            mach_port: AtomicPtr::new(port as _),
            run_loop: AtomicPtr::new(std::ptr::null_mut()),
            is_running: Arc::new(AtomicBool::new(false)),
            _context: context,
        })
    }

    /// Starts the event tap runloop (blocks until stop() or runloop stopped).
    pub fn run(&self) -> Result<(), HookError> {
        let port = self.mach_port.load(Ordering::SeqCst);
        if port.is_null() {
            return Err(HookError::InvalidPort);
        }

        if self
            .is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(HookError::AlreadyRunning);
        }

        struct RunGuard<'a>(&'a AtomicBool, &'a AtomicPtr<std::ffi::c_void>);
        impl Drop for RunGuard<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::SeqCst);
                self.1.store(std::ptr::null_mut(), Ordering::SeqCst);
            }
        }
        let _guard = RunGuard(&self.is_running, &self.run_loop);

        let current_run_loop = unsafe { ffi::CFRunLoopGetCurrent() };
        if current_run_loop.is_null() {
            return Err(HookError::RunLoopError(
                "Failed to get current CFRunLoop".into(),
            ));
        }

        self.run_loop.store(current_run_loop as _, Ordering::SeqCst);

        unsafe {
            let source = ffi::CFMachPortCreateRunLoopSource(std::ptr::null_mut(), port as _, 0);
            if source.is_null() {
                return Err(HookError::RunLoopError(
                    "Failed to create CFRunLoopSource from mach port".into(),
                ));
            }

            ffi::CFRunLoopAddSource(current_run_loop, source, ffi::kCFRunLoopCommonModes);
            ffi::CGEventTapEnable(port as _, true);

            ffi::CFRunLoopRun();

            ffi::CGEventTapEnable(port as _, false);
            ffi::CFRunLoopRemoveSource(current_run_loop, source, ffi::kCFRunLoopCommonModes);
            ffi::CFRelease(source as _);
        }

        Ok(())
    }

    /// Starts the event tap runloop (alias matching PLAN.md).
    pub fn run_loop(&self) -> Result<(), HookError> {
        self.run()
    }

    /// Attaches the event tap to an existing CFRunLoop without blocking.
    pub fn start_with_run_loop(&self, run_loop: ffi::CFRunLoopRef) -> Result<ffi::CFRunLoopSourceRef, HookError> {
        let port = self.mach_port.load(Ordering::SeqCst);
        if port.is_null() {
            return Err(HookError::InvalidPort);
        }

        self.run_loop.store(run_loop as _, Ordering::SeqCst);
        self.is_running.store(true, Ordering::SeqCst);

        unsafe {
            let source = ffi::CFMachPortCreateRunLoopSource(std::ptr::null_mut(), port as _, 0);
            if source.is_null() {
                return Err(HookError::RunLoopError(
                    "Failed to create CFRunLoopSource from mach port".into(),
                ));
            }

            ffi::CFRunLoopAddSource(run_loop, source, ffi::kCFRunLoopCommonModes);
            ffi::CGEventTapEnable(port as _, true);
            Ok(source)
        }
    }

    /// Stops the runloop and disables the tap.
    pub fn stop(&self) {
        let port = self.mach_port.load(Ordering::SeqCst);
        if !port.is_null() {
            unsafe {
                ffi::CGEventTapEnable(port as _, false);
            }
        }

        let rl = self.run_loop.swap(std::ptr::null_mut(), Ordering::SeqCst);
        if !rl.is_null() {
            unsafe {
                ffi::CFRunLoopStop(rl as _);
            }
        }
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// Returns whether the event tap runloop is currently active.
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Checks whether the underlying Mach port is still valid and not invalidated by macOS.
    pub fn is_valid(&self) -> bool {
        let port = self.mach_port.load(Ordering::SeqCst);
        if port.is_null() {
            return false;
        }
        unsafe { ffi::CFMachPortIsValid(port as _) != 0 }
    }

    /// Checks whether the event tap is currently enabled.
    pub fn is_enabled(&self) -> bool {
        let port = self.mach_port.load(Ordering::SeqCst);
        if port.is_null() {
            return false;
        }
        unsafe { ffi::CGEventTapIsEnabled(port as _) }
    }

    /// Attempts to re-enable the tap if it was disabled by timeout.
    pub fn re_enable(&self) -> bool {
        let port = self.mach_port.load(Ordering::SeqCst);
        if port.is_null() {
            return false;
        }
        unsafe {
            if ffi::CFMachPortIsValid(port as _) != 0 {
                ffi::CGEventTapEnable(port as _, true);
                ffi::CGEventTapIsEnabled(port as _)
            } else {
                false
            }
        }
    }
}

impl Drop for EventTapListener {
    fn drop(&mut self) {
        self.stop();
        let port = self.mach_port.swap(std::ptr::null_mut(), Ordering::SeqCst);
        if !port.is_null() {
            unsafe {
                ffi::CFMachPortInvalidate(port as _);
                ffi::CFRelease(port as _);
            }
        }
    }
}
