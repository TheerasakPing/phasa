use rightlang_app::config::AppConfig;
use rightlang_app::rules::ApplicationRuleManager;

fn main() {
    let config = AppConfig::load_or_default(AppConfig::default_config_path());
    let rule_mgr = ApplicationRuleManager::from_config(&config);
    println!(
        "RightLang-RS initialized with default mode: {:?}",
        rule_mgr.default_mode()
    );
}
