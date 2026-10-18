//! Native macOS notification delivery for Phasa.

use std::sync::{Arc, Mutex};

/// Manager responsible for delivering desktop notifications to the user.
#[derive(Debug, Clone)]
pub struct NotificationManager {
    enabled: bool,
    last_notification: Arc<Mutex<Option<(String, String)>>>,
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new(true)
    }
}

impl NotificationManager {
    /// Creates a new `NotificationManager`.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            last_notification: Arc::new(Mutex::new(None)),
        }
    }

    /// Whether notifications are currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enables or disables notifications.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns the last notification `(title, body)` sent by this manager, if any.
    pub fn last_notification(&self) -> Option<(String, String)> {
        self.last_notification.lock().ok()?.clone()
    }

    /// Clears the recorded last notification.
    pub fn clear_last_notification(&self) {
        if let Ok(mut lock) = self.last_notification.lock() {
            *lock = None;
        }
    }

    /// Delivers an auto-correction notification.
    ///
    /// Title: "Phasa (ภาษา)"
    /// Body: format!("{} → {}", original.trim(), replacement.trim())
    pub fn notify_correction(&self, original: &str, replacement: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let title = "Phasa (ภาษา)";
        let body = format!("{} → {}", original.trim(), replacement.trim());
        self.send_notification(title, &body)
    }

    /// Sends a general notification with title and body.
    pub fn send_notification(&self, title: &str, message: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Record last notification for inspection / testing
        if let Ok(mut lock) = self.last_notification.lock() {
            *lock = Some((title.to_string(), message.to_string()));
        }

        #[cfg(test)]
        {
            // Avoid blocking on external processes during unit tests
            true
        }

        #[cfg(not(test))]
        {
            // 1. Try mac-notification-sys
            #[cfg(target_os = "macos")]
            {
                if mac_notification_sys::send_notification(title, None, message, None).is_ok() {
                    return true;
                }

                // 2. Fallback to osascript if mac-notification-sys fails (e.g. CLI without bundle ID)
                if Self::send_via_osascript(title, message) {
                    return true;
                }
            }

            // 3. Fallback for headless / unsupported environments: log to stderr
            eprintln!("[Notification] {}: {}", title, message);
            true
        }
    }

    /// Sends a notification via macOS osascript.
    #[cfg(target_os = "macos")]
    pub fn send_via_osascript(title: &str, message: &str) -> bool {
        let escaped_title = title.replace('\\', "\\\\").replace('\"', "\\\"");
        let escaped_message = message.replace('\\', "\\\\").replace('\"', "\\\"");
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            escaped_message, escaped_title
        );

        match std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
        {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn send_via_osascript(_title: &str, _message: &str) -> bool {
        false
    }
}
