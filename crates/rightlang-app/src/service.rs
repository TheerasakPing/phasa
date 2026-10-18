//! LaunchAgent daemon service management for macOS.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::AppConfig;

pub const LAUNCHD_SERVICE_NAME: &str = "com.phasa.daemon";
pub const LAUNCHD_PLIST_FILENAME: &str = "com.phasa.daemon.plist";

/// Returns the standard path to user's LaunchAgents directory (`~/Library/LaunchAgents`).
pub fn launch_agents_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("Library").join("LaunchAgents"))
}

/// Returns the full path to the Phasa LaunchAgent plist file.
pub fn launchd_plist_path() -> Option<PathBuf> {
    launch_agents_dir().map(|d| d.join(LAUNCHD_PLIST_FILENAME))
}

/// Checks whether the Phasa LaunchAgent plist file currently exists.
pub fn is_service_installed() -> bool {
    launchd_plist_path().map(|p| p.exists()).unwrap_or(false)
}

/// Generates the launchd XML plist content for the given executable, log, and error log paths.
pub fn generate_plist_xml(exe_path: &Path, log_path: &Path, err_log_path: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{stdout}</string>
    <key>StandardErrorPath</key>
    <string>{stderr}</string>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
"#,
        label = LAUNCHD_SERVICE_NAME,
        exe = exe_path.display(),
        stdout = log_path.display(),
        stderr = err_log_path.display(),
    )
}

/// Installs and loads the launchd agent at custom paths (useful for unit testing).
pub fn install_launchd_service_at(
    plist_path: &Path,
    exe_path: &Path,
    config_dir: &Path,
    run_launchctl: bool,
) -> Result<PathBuf, String> {
    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {:?}: {}", parent, e))?;
    }
    fs::create_dir_all(config_dir)
        .map_err(|e| format!("Failed to create config dir {:?}: {}", config_dir, e))?;

    let log_path = config_dir.join("daemon.log");
    let err_log_path = config_dir.join("daemon.err.log");

    let plist_content = generate_plist_xml(exe_path, &log_path, &err_log_path);
    fs::write(plist_path, plist_content)
        .map_err(|e| format!("Failed to write plist to {:?}: {}", plist_path, e))?;

    if run_launchctl {
        let _ = Command::new("launchctl")
            .arg("unload")
            .arg(plist_path)
            .output();
        let load_res = Command::new("launchctl")
            .arg("load")
            .arg(plist_path)
            .output();
        if let Err(e) = load_res {
            eprintln!("Warning: launchctl load failed: {}", e);
        }
    }

    Ok(plist_path.to_path_buf())
}

/// Installs and loads the Phasa daemon LaunchAgent for the current user.
pub fn install_launchd_service() -> Result<PathBuf, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Cannot resolve executable path: {}", e))?;
    let canonical_exe = fs::canonicalize(&current_exe).unwrap_or(current_exe);

    let plist_path =
        launchd_plist_path().ok_or_else(|| "Cannot resolve user home directory".to_string())?;
    let config_dir = AppConfig::default_config_dir();

    install_launchd_service_at(&plist_path, &canonical_exe, &config_dir, true)
}

/// Uninstalls and removes the launchd agent at custom paths (useful for testing).
pub fn uninstall_launchd_service_at(plist_path: &Path, run_launchctl: bool) -> Result<(), String> {
    if plist_path.exists() {
        if run_launchctl {
            let _ = Command::new("launchctl")
                .arg("unload")
                .arg(plist_path)
                .output();
        }
        fs::remove_file(plist_path)
            .map_err(|e| format!("Failed to remove plist {:?}: {}", plist_path, e))?;
    }
    Ok(())
}

/// Uninstalls and removes the Phasa daemon LaunchAgent.
pub fn uninstall_launchd_service() -> Result<(), String> {
    let plist_path =
        launchd_plist_path().ok_or_else(|| "Cannot resolve user home directory".to_string())?;
    uninstall_launchd_service_at(&plist_path, true)
}

/// Toggles the Phasa LaunchAgent.
/// Returns `Ok(true)` if newly installed, or `Ok(false)` if uninstalled.
pub fn toggle_launchd_service() -> Result<bool, String> {
    if is_service_installed() {
        uninstall_launchd_service()?;
        Ok(false)
    } else {
        install_launchd_service()?;
        Ok(true)
    }
}
