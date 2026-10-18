//! Native macOS menu bar status item and tray menu management for Phasa.

use muda::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::config::CorrectionMode;

/// Actions emitted from tray menu item selections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// Update the active correction mode.
    SetMode(CorrectionMode),
    /// Toggle Start at Login LaunchAgent service.
    ToggleStartAtLogin,
    /// Open the Phasa configuration folder in Finder / file manager.
    OpenConfigDir,
    /// Terminate the daemon gracefully.
    Quit,
}

/// Manages the native macOS status bar menu item and interactive dropdown menu.
pub struct TrayManager {
    #[allow(dead_code)]
    tray_icon: Option<TrayIcon>,
    mode_item: Option<MenuItem>,
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
                start_login_item: None,
                current_mode: initial_mode,
            });
        }

        let menu = Menu::new();

        let header_item = MenuItem::with_id("app_header", "Phasa (ภาษา) v0.1.0", false, None);
        let initial_text = format!("Mode: {}", Self::mode_display_name(initial_mode));
        let mode_item = MenuItem::with_id("mode_info", initial_text, false, None);

        let sep1 = PredefinedMenuItem::separator();
        let item_auto = MenuItem::with_id("set_auto", "Auto (อัตโนมัติ)", true, None);
        let item_manual = MenuItem::with_id("set_manual", "Manual (แปลงเฉพาะที่เลือก)", true, None);
        let item_disabled = MenuItem::with_id("set_disabled", "Disabled (ปิดชั่วคราว)", true, None);

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

    /// Sets the macOS application activation policy to Accessory (menubar-only, no Dock icon).
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
            type MsgSendNoArgs = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
            type MsgSendIsize = unsafe extern "C" fn(*mut c_void, *mut c_void, isize) -> bool;

            let msg_send: MsgSendNoArgs = std::mem::transmute(objc_msgSend as *const ());
            let msg_send_isize: MsgSendIsize = std::mem::transmute(objc_msgSend as *const ());

            let cls_nsapp = objc_getClass(c"NSApplication".as_ptr());
            if !cls_nsapp.is_null() {
                let sel_shared = sel_registerName(c"sharedApplication".as_ptr());
                let app = msg_send(cls_nsapp, sel_shared);
                if !app.is_null() {
                    let sel_policy = sel_registerName(c"setActivationPolicy:".as_ptr());
                    // NSApplicationActivationPolicyAccessory = 1
                    let _ = msg_send_isize(app, sel_policy, 1);
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

    /// Maps a menu item identifier to a `TrayAction`.
    pub fn action_from_id(id: &str) -> Option<TrayAction> {
        match id {
            "set_auto" => Some(TrayAction::SetMode(CorrectionMode::Auto)),
            "set_manual" => Some(TrayAction::SetMode(CorrectionMode::Manual)),
            "set_disabled" => Some(TrayAction::SetMode(CorrectionMode::Disabled)),
            "toggle_start_login" => Some(TrayAction::ToggleStartAtLogin),
            "open_config" => Some(TrayAction::OpenConfigDir),
            "quit" => Some(TrayAction::Quit),
            _ => None,
        }
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
                        rgba.extend_from_slice(&[0, 0, 0, 0]);
                    }
                } else {
                    rgba.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }

        Icon::from_rgba(rgba, WIDTH, HEIGHT).ok()
    }
}
