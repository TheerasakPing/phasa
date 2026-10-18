//! Native macOS AppKit Dashboard & Settings Window for Phasa.

use std::ffi::CString;
use std::os::raw::c_char;

use crate::config::CorrectionMode;

extern "C" {
    fn phasa_ui_init(initial_mode: i32, is_autostart: bool, is_notify: bool);
    fn phasa_ui_show_window();
    fn phasa_ui_hide_window();
    fn phasa_ui_is_window_visible() -> bool;
    fn phasa_ui_set_accessibility_status(granted: bool);
    fn phasa_ui_set_playground_result(result_text: *const c_char);
    fn phasa_ui_set_mode(mode: i32);
    fn phasa_ui_set_autostart(enabled: bool);
    fn phasa_ui_run_app();
    fn phasa_ui_stop_app();

    fn phasa_ui_register_callbacks(
        playground_cb: extern "C" fn(*const c_char),
        mode_cb: extern "C" fn(i32),
        autostart_cb: extern "C" fn(bool),
        notify_cb: extern "C" fn(bool),
        open_config_cb: extern "C" fn(),
        open_access_cb: extern "C" fn(),
        poll_timer_cb: extern "C" fn(),
    );
}

pub struct DashboardUI;

impl DashboardUI {
    pub fn init(mode: CorrectionMode, is_autostart: bool, is_notify: bool) {
        let m = match mode {
            CorrectionMode::Auto => 0,
            CorrectionMode::Manual => 1,
            CorrectionMode::Disabled => 2,
        };
        unsafe {
            phasa_ui_init(m, is_autostart, is_notify);
        }
    }

    pub fn show() {
        unsafe {
            phasa_ui_show_window();
        }
    }

    pub fn hide() {
        unsafe {
            phasa_ui_hide_window();
        }
    }

    pub fn is_visible() -> bool {
        unsafe { phasa_ui_is_window_visible() }
    }

    pub fn set_accessibility_status(granted: bool) {
        unsafe {
            phasa_ui_set_accessibility_status(granted);
        }
    }

    pub fn set_playground_result(result: &str) {
        if let Ok(c_str) = CString::new(result) {
            unsafe {
                phasa_ui_set_playground_result(c_str.as_ptr());
            }
        }
    }

    pub fn set_mode(mode: CorrectionMode) {
        let m = match mode {
            CorrectionMode::Auto => 0,
            CorrectionMode::Manual => 1,
            CorrectionMode::Disabled => 2,
        };
        unsafe {
            phasa_ui_set_mode(m);
        }
    }

    pub fn set_autostart(enabled: bool) {
        unsafe {
            phasa_ui_set_autostart(enabled);
        }
    }

    pub fn run() {
        unsafe {
            phasa_ui_run_app();
        }
    }

    pub fn stop() {
        unsafe {
            phasa_ui_stop_app();
        }
    }

    pub fn register_callbacks(
        playground_cb: extern "C" fn(*const c_char),
        mode_cb: extern "C" fn(i32),
        autostart_cb: extern "C" fn(bool),
        notify_cb: extern "C" fn(bool),
        open_config_cb: extern "C" fn(),
        open_access_cb: extern "C" fn(),
        poll_timer_cb: extern "C" fn(),
    ) {
        unsafe {
            phasa_ui_register_callbacks(
                playground_cb,
                mode_cb,
                autostart_cb,
                notify_cb,
                open_config_cb,
                open_access_cb,
                poll_timer_cb,
            );
        }
    }
}
