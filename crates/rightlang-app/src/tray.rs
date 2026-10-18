//! System menubar tray icon and interactive menu management using `tray-icon` and `muda`.

use std::sync::atomic::{AtomicBool, Ordering};

use muda::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::config::CorrectionMode;

static IS_RUNNING: AtomicBool = AtomicBool::new(true);

/// Action triggered by user interacting with the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// Open the native dashboard window.
    OpenDashboard,
    /// Switch the active correction mode.
    SetMode(CorrectionMode),
    /// Toggle autostart (launchd agent).
    ToggleStartAtLogin,
    /// Open configuration directory in Finder.
    OpenConfigDir,
    /// Open macOS accessibility settings in System Preferences.
    OpenAccessibility,
    /// Terminate the daemon gracefully.
    Quit,
}

/// Manages the native macOS status bar menu item and interactive dropdown menu.
pub struct TrayManager {
    #[allow(dead_code)]
    tray_icon: Option<TrayIcon>,
    mode_item: Option<MenuItem>,
    perm_item: Option<MenuItem>,
    auto_item: Option<CheckMenuItem>,
    manual_item: Option<CheckMenuItem>,
    disabled_item: Option<CheckMenuItem>,
    start_login_item: Option<CheckMenuItem>,
    current_mode: CorrectionMode,
}

impl TrayManager {
    /// Initializes the tray status item with the given initial correction mode.
    pub fn new(
        initial_mode: CorrectionMode,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // AppKit NSMenu can only be initialized on the main OS thread
        if !Self::is_main_thread() {
            return Ok(Self {
                tray_icon: None,
                mode_item: None,
                perm_item: None,
                auto_item: None,
                manual_item: None,
                disabled_item: None,
                start_login_item: None,
                current_mode: initial_mode,
            });
        }

        let menu = Menu::new();

        let header_item = MenuItem::with_id("app_header", "Phasa (ภาษา) v0.1.0", false, None);
        let item_dashboard = MenuItem::with_id(
            "open_dashboard",
            "Open Dashboard & Settings (เปิดหน้าต่างควบคุม)...",
            true,
            None,
        );
        let item_perm = MenuItem::with_id(
            "open_accessibility",
            "⚠️ Grant Accessibility Permission (ให้สิทธิ์การเข้าถึง)",
            true,
            None,
        );
        let initial_text = format!("Mode: {}", Self::mode_display_name(initial_mode));
        let mode_item = MenuItem::with_id("mode_info", initial_text, false, None);

        let sep1 = PredefinedMenuItem::separator();
        let item_auto = CheckMenuItem::with_id(
            "set_auto",
            "Auto (อัตโนมัติ)",
            true,
            initial_mode == CorrectionMode::Auto,
            None,
        );
        let item_manual = CheckMenuItem::with_id(
            "set_manual",
            "Manual (แปลงเฉพาะที่เลือก)",
            true,
            initial_mode == CorrectionMode::Manual,
            None,
        );
        let item_disabled = CheckMenuItem::with_id(
            "set_disabled",
            "Disabled (ปิดชั่วคราว)",
            true,
            initial_mode == CorrectionMode::Disabled,
            None,
        );

        let sep2 = PredefinedMenuItem::separator();
        let is_service_installed = crate::service::is_service_installed();
        let item_start_login = CheckMenuItem::with_id(
            "toggle_start_login",
            "Start at Login (เปิดพร้อมเครื่อง)",
            true,
            is_service_installed,
            None,
        );

        let sep3 = PredefinedMenuItem::separator();
        let item_open_config =
            MenuItem::with_id("open_config", "Open Config Directory (~/.config/phasa)", true, None);
        let item_quit = MenuItem::with_id("quit", "Quit Phasa", true, None);

        menu.append_items(&[
            &header_item,
            &item_dashboard,
            &item_perm,
            &mode_item,
            &sep1,
            &item_auto,
            &item_manual,
            &item_disabled,
            &sep2,
            &item_start_login,
            &sep3,
            &item_open_config,
            &item_quit,
        ])?;

        let icon = Self::generate_default_icon();
        let mut builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_title("ภาษา")
            .with_tooltip(format!(
                "Phasa (ภาษา) - {}",
                Self::mode_display_name(initial_mode)
            ));

        if let Some(ic) = icon {
            builder = builder.with_icon(ic);
        }

        let tray_icon = match builder.build() {
            Ok(t) => Some(t),
            Err(err) => {
                eprintln!(
                    "Warning: Failed to create system tray icon: {}. Continuing in headless/daemon mode.",
                    err
                );
                None
            }
        };

        Ok(Self {
            tray_icon,
            mode_item: Some(mode_item),
            perm_item: Some(item_perm),
            auto_item: Some(item_auto),
            manual_item: Some(item_manual),
            disabled_item: Some(item_disabled),
            start_login_item: Some(item_start_login),
            current_mode: initial_mode,
        })
    }

    /// Checks if the current thread is the main operating system thread.
    #[cfg(target_os = "macos")]
    pub fn is_main_thread() -> bool {
        use std::ffi::c_void;
        use std::os::raw::c_char;

        #[link(name = "Foundation", kind = "framework")]
        #[link(name = "objc", kind = "dylib")]
        extern "C" {
            fn objc_getClass(name: *const c_char) -> *mut c_void;
            fn sel_registerName(name: *const c_char) -> *mut c_void;
            fn objc_msgSend();
        }

        unsafe {
            type MsgSendBool = unsafe extern "C" fn(*mut c_void, *mut c_void) -> bool;
            let msg_send: MsgSendBool = std::mem::transmute(objc_msgSend as *const ());
            let cls = objc_getClass(c"NSThread".as_ptr());
            if cls.is_null() {
                return false;
            }
            let sel = sel_registerName(c"isMainThread".as_ptr());
            msg_send(cls, sel)
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn is_main_thread() -> bool {
        true
    }

    /// Sets up macOS Dock and activation policy to run as an accessory/background daemon without a Dock icon.
    #[cfg(target_os = "macos")]
    pub fn setup_dock_policy() {
        if !Self::is_main_thread() {
            return;
        }
        use std::ffi::c_void;
        use std::os::raw::c_char;

        #[link(name = "AppKit", kind = "framework")]
        #[link(name = "objc", kind = "dylib")]
        extern "C" {
            fn objc_getClass(name: *const c_char) -> *mut c_void;
            fn sel_registerName(name: *const c_char) -> *mut c_void;
            fn objc_msgSend();
        }

        unsafe {
            type MsgSendVoid = unsafe extern "C" fn(*mut c_void, *mut c_void);
            type MsgSendArg = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);
            type MsgSendBool = unsafe extern "C" fn(*mut c_void, *mut c_void, i64) -> bool;
            type MsgSendStr =
                unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_char) -> *mut c_void;
            type MsgSendObj = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;

            let msg_send: MsgSendObj = std::mem::transmute(objc_msgSend as *const ());
            let msg_send_bool: MsgSendBool = std::mem::transmute(objc_msgSend as *const ());
            let msg_send_void: MsgSendVoid = std::mem::transmute(objc_msgSend as *const ());
            let msg_send_arg: MsgSendArg = std::mem::transmute(objc_msgSend as *const ());
            let msg_send_str: MsgSendStr = std::mem::transmute(objc_msgSend as *const ());

            // NSApplicationActivationPolicyAccessory = 1
            let cls_app = objc_getClass(c"NSApplication".as_ptr());
            if !cls_app.is_null() {
                let sel_shared = sel_registerName(c"sharedApplication".as_ptr());
                let app = msg_send(cls_app, sel_shared);
                if !app.is_null() {
                    let sel_set_policy = sel_registerName(c"setActivationPolicy:".as_ptr());
                    msg_send_bool(app, sel_set_policy, 1);
                }
            }

            // Disable macOS Automatic Termination & Sudden Termination for this persistent background daemon
            let cls_process_info = objc_getClass(c"NSProcessInfo".as_ptr());
            if !cls_process_info.is_null() {
                let sel_process_info = sel_registerName(c"processInfo".as_ptr());
                let p_info = msg_send(cls_process_info, sel_process_info);
                if !p_info.is_null() {
                    let cls_nsstring = objc_getClass(c"NSString".as_ptr());
                    let sel_str = sel_registerName(c"stringWithUTF8String:".as_ptr());
                    let reason = msg_send_str(cls_nsstring, sel_str, c"Phasa persistent input daemon".as_ptr());

                    let sel_disable_auto = sel_registerName(c"disableAutomaticTermination:".as_ptr());
                    msg_send_arg(p_info, sel_disable_auto, reason);

                    let sel_disable_sudden = sel_registerName(c"disableSuddenTermination".as_ptr());
                    msg_send_void(p_info, sel_disable_sudden);
                }
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn setup_dock_policy() {}

    /// Returns the currently active correction mode represented by the tray.
    pub fn current_mode(&self) -> CorrectionMode {
        self.current_mode
    }

    /// Updates the displayed correction mode in the dropdown menu and tooltip.
    pub fn set_mode(&mut self, mode: CorrectionMode) {
        self.current_mode = mode;
        let display = Self::mode_display_name(mode);
        if let Some(ref item) = self.mode_item {
            item.set_text(format!("Mode: {}", display));
        }
        if let Some(ref item) = self.auto_item {
            item.set_checked(mode == CorrectionMode::Auto);
        }
        if let Some(ref item) = self.manual_item {
            item.set_checked(mode == CorrectionMode::Manual);
        }
        if let Some(ref item) = self.disabled_item {
            item.set_checked(mode == CorrectionMode::Disabled);
        }
        if let Some(tray) = &mut self.tray_icon {
            let _ = tray.set_tooltip(Some(format!("Phasa (ภาษา) - {}", display)));
        }
    }

    /// Returns whether the Start at Login checkbox is currently checked in the tray.
    pub fn is_start_at_login(&self) -> bool {
        self.start_login_item
            .as_ref()
            .map(|i| i.is_checked())
            .unwrap_or(false)
    }

    /// Updates the checkmark state for the Start at Login menu item.
    pub fn set_start_at_login(&mut self, enabled: bool) {
        if let Some(ref item) = self.start_login_item {
            item.set_checked(enabled);
        }
    }

    /// Polls for user interactions from the tray dropdown menu.
    pub fn check_action(&self) -> Option<TrayAction> {
        if let Ok(event) = muda::MenuEvent::receiver().try_recv() {
            Self::action_from_id(event.id().as_ref())
        } else {
            None
        }
    }

    /// Maps a menu item identifier to a `TrayAction` LOGIC.
    pub fn action_from_id(id: &str) -> Option<TrayAction> {
        match id {
            "open_dashboard" => Some(TrayAction::OpenDashboard),
            "set_auto" => Some(TrayAction::SetMode(CorrectionMode::Auto)),
            "set_manual" => Some(TrayAction::SetMode(CorrectionMode::Manual)),
            "set_disabled" => Some(TrayAction::SetMode(CorrectionMode::Disabled)),
            "toggle_start_login" => Some(TrayAction::ToggleStartAtLogin),
            "open_config" => Some(TrayAction::OpenConfigDir),
            "open_accessibility" => Some(TrayAction::OpenAccessibility),
            "quit" => Some(TrayAction::Quit),
            _ => None,
        }
    }

    /// Updates the accessibility permission notice in the tray menu.
    pub fn set_accessibility_status(&mut self, granted: bool) {
        if let Some(ref item) = self.perm_item {
            if granted {
                item.set_text("✅ Accessibility Permission Granted");
                item.set_enabled(false);
            } else {
                item.set_text("⚠️ Grant Accessibility Permission (คลิกเพื่อให้สิทธิ์)");
                item.set_enabled(true);
            }
        }
        if granted {
            self.set_mode(self.current_mode);
        } else {
            if let Some(ref item) = self.mode_item {
                item.set_text("Status: ⚠️ Waiting for Accessibility Permission");
            }
            if let Some(tray) = &mut self.tray_icon {
                let _ = tray.set_tooltip(Some("Phasa (ภาษา) - Waiting for Accessibility Permission".to_string()));
            }
        }
    }

    /// Opens macOS Accessibility System Settings.
    pub fn open_accessibility_settings() -> std::io::Result<()> {
        #[cfg(all(target_os = "macos", not(test)))]
        {
            std::process::Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                .spawn()?;
        }
        Ok(())
    }

    /// Opens the configuration directory in Finder.
    pub fn open_config_dir() -> std::io::Result<()> {
        let config_path = crate::config::AppConfig::default_config_path();
        let dir = config_path.parent().unwrap_or(&config_path);
        let _ = std::fs::create_dir_all(dir);

        #[cfg(all(target_os = "macos", not(test)))]
        {
            std::process::Command::new("open").arg(dir).spawn()?;
        }
        Ok(())
    }

    fn mode_display_name(mode: CorrectionMode) -> &'static str {
        match mode {
            CorrectionMode::Auto => "Auto",
            CorrectionMode::Manual => "Manual",
            CorrectionMode::Disabled => "Disabled",
        }
    }

    fn generate_default_icon() -> Option<Icon> {
        const WIDTH: u32 = 16;
        const HEIGHT: u32 = 16;
        let mut rgba = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let dx = (x as f32) - 7.5;
                let dy = (y as f32) - 7.5;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist <= 6.5 {
                    if dist >= 5.0 {
                        // Cyan/violet neon border ring
                        rgba.extend_from_slice(&[56, 189, 248, 255]);
                    } else if (3..=12).contains(&x) && (3..=12).contains(&y) {
                        // Sleek deep violet-cyan fill
                        rgba.extend_from_slice(&[14, 165, 233, 255]);
                    } else {
                        rgba.extend_from_slice(&[30, 41, 59, 255]);
                    }
                } else {
                    // Transparent outer edge
                    rgba.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }

        Icon::from_rgba(rgba, WIDTH, HEIGHT).ok()
    }

    /// Checks if the application should keep running.
    pub fn is_running() -> bool {
        IS_RUNNING.load(Ordering::SeqCst)
    }

    /// Requests shutdown of the application.
    pub fn stop() {
        IS_RUNNING.store(false, Ordering::SeqCst);
    }
}
