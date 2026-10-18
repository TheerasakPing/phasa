//! Application configuration, rule management, and daemon coordination for RightLang.

pub mod config;
pub mod rules;

pub use config::{AppConfig, CorrectionMode};
pub use rules::{get_frontmost_app_bundle_id, ApplicationRuleManager};
