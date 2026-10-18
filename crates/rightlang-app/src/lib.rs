//! Application configuration, rule management, and daemon coordination for Phasa.

pub mod config;
pub mod notify;
pub mod rules;
pub mod service;
pub mod tray;
pub mod ui;
pub mod worker;

pub use config::{AppConfig, CorrectionMode};
pub use notify::NotificationManager;
pub use rules::{get_frontmost_app_bundle_id, ApplicationRuleManager};
pub use tray::{TrayAction, TrayManager};
pub use ui::DashboardUI;
pub use worker::{load_dictionaries, CorrectionRecord, KeyWorker};
