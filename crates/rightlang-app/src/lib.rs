//! Application configuration, rule management, and daemon coordination for RightLang.

pub mod config;
pub mod rules;
pub mod worker;

pub use config::{AppConfig, CorrectionMode};
pub use rules::{get_frontmost_app_bundle_id, ApplicationRuleManager};
pub use worker::{load_dictionaries, CorrectionRecord, KeyWorker};
