//! Configuration management for Phasa.

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Correction mode for Phasa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CorrectionMode {
    /// Automatically detect and correct mistyped words.
    #[default]
    Auto,
    /// Only convert when triggered manually by hotkey.
    Manual,
    /// Disabled completely for the app or globally.
    Disabled,
}

impl CorrectionMode {
    /// Returns the string representation of the correction mode.
    pub const fn as_str(&self) -> &'static str {
        match self {
            CorrectionMode::Auto => "auto",
            CorrectionMode::Manual => "manual",
            CorrectionMode::Disabled => "disabled",
        }
    }
}

impl fmt::Display for CorrectionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

fn default_mode() -> CorrectionMode {
    CorrectionMode::Auto
}

const fn default_key_delay_us() -> u64 {
    1000
}

const fn default_true() -> bool {
    true
}

fn default_app_rules() -> HashMap<String, CorrectionMode> {
    let mut rules = HashMap::new();
    // Terminal applications where auto-correction causes disruption
    rules.insert("com.apple.Terminal".to_string(), CorrectionMode::Disabled);
    rules.insert("com.googlecode.iterm2".to_string(), CorrectionMode::Disabled);
    rules.insert("net.kovidgoyal.kitty".to_string(), CorrectionMode::Disabled);
    rules.insert("com.github.wez.wezterm".to_string(), CorrectionMode::Disabled);
    rules.insert("dev.warp.Warp-GDK".to_string(), CorrectionMode::Disabled);
    rules.insert("co.zeit.hyper".to_string(), CorrectionMode::Disabled);

    // Password managers
    rules.insert("com.agilebits.onepassword".to_string(), CorrectionMode::Disabled);
    rules.insert("com.1password.1password".to_string(), CorrectionMode::Disabled);
    rules.insert("com.bitwarden.desktop".to_string(), CorrectionMode::Disabled);
    rules.insert("org.keepassxc.keepassxc".to_string(), CorrectionMode::Disabled);

    rules
}

/// Main application configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    /// Global default correction mode.
    #[serde(default = "default_mode")]
    pub mode: CorrectionMode,

    /// Delay in microseconds between synthetic keystrokes (e.g. backspaces and replacements).
    #[serde(default = "default_key_delay_us")]
    pub key_delay_us: u64,

    /// Whether to learn new words automatically from user corrections.
    #[serde(default = "default_true")]
    pub auto_learn: bool,

    /// Whether to display a notification when text is automatically corrected.
    #[serde(default = "default_true")]
    pub notify_on_correction: bool,

    /// Whether to display a notification when a new word is learned.
    #[serde(default = "default_true")]
    pub notify_on_learn: bool,

    /// Per-application correction mode overrides keyed by bundle identifier.
    #[serde(default = "default_app_rules")]
    pub app_rules: HashMap<String, CorrectionMode>,

    /// Optional hotkey to manually trigger conversion of recent buffer.
    #[serde(default)]
    pub hotkey_switch: Option<String>,

    /// Optional hotkey to cycle correction modes.
    #[serde(default)]
    pub hotkey_mode: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            key_delay_us: default_key_delay_us(),
            auto_learn: default_true(),
            notify_on_correction: default_true(),
            notify_on_learn: default_true(),
            app_rules: default_app_rules(),
            hotkey_switch: None,
            hotkey_mode: None,
        }
    }
}

impl AppConfig {
    /// Loads configuration from the given file path, or returns default configuration
    /// if the file does not exist or cannot be parsed.
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        let target_path = if !path.exists() {
            // Check legacy fallback path if loading from default path
            if let Some(home) = dirs::home_dir() {
                let legacy = home.join(".config").join("rightlang").join("config.toml");
                if legacy.exists() {
                    legacy
                } else {
                    return Self::default();
                }
            } else {
                return Self::default();
            }
        } else {
            path.to_path_buf()
        };

        match fs::read_to_string(&target_path) {
            Ok(content) => match toml::from_str::<AppConfig>(&content) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!(
                        "Warning: Failed to parse config at {:?}: {}. Using defaults.",
                        target_path, err
                    );
                    Self::default()
                }
            },
            Err(err) => {
                eprintln!(
                    "Warning: Failed to read config at {:?}: {}. Using defaults.",
                    target_path, err
                );
                Self::default()
            }
        }
    }

    /// Saves the configuration to the specified file in TOML format,
    /// creating parent directories if necessary.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = toml::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, serialized)
    }

    /// Returns the default configuration directory (~/.config/phasa).
    pub fn default_config_dir() -> PathBuf {
        default_config_dir()
    }

    /// Returns the default configuration path (~/.config/phasa/config.toml).
    pub fn default_config_path() -> PathBuf {
        default_config_dir().join("config.toml")
    }

    /// Returns the default path for custom Thai user dictionary (~/.config/phasa/user.th.txt).
    pub fn default_user_thai_dict_path() -> PathBuf {
        default_user_thai_dict_path()
    }

    /// Returns the default path for custom English user dictionary (~/.config/phasa/user.en.txt).
    pub fn default_user_eng_dict_path() -> PathBuf {
        default_user_eng_dict_path()
    }
}

/// Returns the default configuration directory (~/.config/phasa).
pub fn default_config_dir() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".config").join("phasa")
    } else {
        PathBuf::from(".config/phasa")
    }
}

/// Returns the default path for custom Thai user dictionary (~/.config/phasa/user.th.txt).
pub fn default_user_thai_dict_path() -> PathBuf {
    let p = default_config_dir().join("user.th.txt");
    if !p.exists() {
        if let Some(home) = dirs::home_dir() {
            let legacy = home.join(".config").join("rightlang").join("user.th.txt");
            if legacy.exists() {
                return legacy;
            }
        }
    }
    p
}

/// Returns the default path for custom English user dictionary (~/.config/phasa/user.en.txt).
pub fn default_user_eng_dict_path() -> PathBuf {
    let p = default_config_dir().join("user.en.txt");
    if !p.exists() {
        if let Some(home) = dirs::home_dir() {
            let legacy = home.join(".config").join("rightlang").join("user.en.txt");
            if legacy.exists() {
                return legacy;
            }
        }
    }
    p
}
