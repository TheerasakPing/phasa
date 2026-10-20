//! Intelligent per-application input source memory and auto-switching.

use std::collections::{HashMap, HashSet};
use rightlang_macos::tis::InputSourceManager;

/// Manages remembered input source IDs per application bundle identifier.
#[derive(Debug, Clone)]
pub struct AppLanguageManager {
    memory: HashMap<String, String>,
    developer_apps: HashSet<String>,
    input_source_manager: InputSourceManager,
    last_active_app: Option<String>,
}

impl AppLanguageManager {
    /// Creates a new `AppLanguageManager` pre-configured with known developer/terminal applications.
    pub fn new() -> Self {
        let mut developer_apps = HashSet::new();
        // Common terminal and code editor bundle identifiers on macOS
        developer_apps.insert("com.apple.Terminal".to_string());
        developer_apps.insert("com.googlecode.iterm2".to_string());
        developer_apps.insert("com.microsoft.VSCode".to_string());
        developer_apps.insert("com.microsoft.VSCodeInsiders".to_string());
        developer_apps.insert("com.todesktop.230313mzl4w4u92".to_string()); // Cursor
        developer_apps.insert("com.apple.dt.Xcode".to_string());
        developer_apps.insert("dev.warp.Warp-Stable".to_string());
        developer_apps.insert("com.sublimetext.4".to_string());
        developer_apps.insert("com.sublimetext.3".to_string());
        developer_apps.insert("com.jetbrains.intellij".to_string());
        developer_apps.insert("com.jetbrains.pycharm".to_string());
        developer_apps.insert("com.jetbrains.rustrover".to_string());
        developer_apps.insert("com.jetbrains.WebStorm".to_string());
        developer_apps.insert("com.github.wez.wezterm".to_string());
        developer_apps.insert("net.kovidgoyal.kitty".to_string());
        developer_apps.insert("org.alacritty".to_string());

        Self {
            memory: HashMap::new(),
            developer_apps,
            input_source_manager: InputSourceManager::new(),
            last_active_app: None,
        }
    }

    /// Handles an active application change event (e.g. from polling or workspace notification).
    ///
    /// Saves the current input source for the outgoing application,
    /// and restores the remembered or default input source for the incoming application.
    pub fn on_app_activated(&mut self, new_bundle_id: &str) {
        let trimmed_new = new_bundle_id.trim();
        if trimmed_new.is_empty() {
            return;
        }

        // If app hasn't actually changed, nothing to do
        if let Some(ref last) = self.last_active_app {
            if last == trimmed_new {
                return;
            }

            // Save outgoing app's active input source
            if let Ok(current_source) = self.input_source_manager.get_current_source_id() {
                self.memory.insert(last.clone(), current_source);
            }
        }

        // Check if incoming app has a remembered input source
        if let Some(remembered_source) = self.memory.get(trimmed_new).cloned() {
            let _ = self.input_source_manager.select_input_source(&remembered_source);
        } else if self.developer_apps.contains(trimmed_new) {
            // For developer/terminal apps with no prior memory, default to English (ABC)
            let _ = self.input_source_manager.switch_to_english();
        }

        self.last_active_app = Some(trimmed_new.to_string());
    }

    /// Explicitly sets a remembered input source for an application.
    pub fn set_app_source(&mut self, bundle_id: &str, source_id: &str) {
        self.memory.insert(bundle_id.trim().to_string(), source_id.to_string());
    }

    /// Returns the remembered input source for an application, if any.
    pub fn get_app_source(&self, bundle_id: &str) -> Option<&str> {
        self.memory.get(bundle_id.trim()).map(|s| s.as_str())
    }

    /// Returns whether an application is recognized as a developer / terminal application.
    pub fn is_developer_app(&self, bundle_id: &str) -> bool {
        self.developer_apps.contains(bundle_id.trim())
    }

    /// Returns the currently tracked frontmost app ID.
    pub fn last_active_app(&self) -> Option<&str> {
        self.last_active_app.as_deref()
    }
}

impl Default for AppLanguageManager {
    fn default() -> Self {
        Self::new()
    }
}
