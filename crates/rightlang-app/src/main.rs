use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use core_foundation_sys::base::{kCFAllocatorDefault, CFRelease};
use core_foundation_sys::date::CFAbsoluteTimeGetCurrent;
use core_foundation_sys::runloop::{
    kCFRunLoopCommonModes, CFRunLoopAddTimer, CFRunLoopGetCurrent, CFRunLoopRemoveTimer,
    CFRunLoopTimerContext, CFRunLoopTimerCreate, CFRunLoopTimerInvalidate, CFRunLoopTimerRef,
};
use crossbeam_channel::unbounded;
use rightlang_app::config::{AppConfig, CorrectionMode};
use rightlang_app::notify::NotificationManager;
use rightlang_app::tray::{TrayAction, TrayManager};
use rightlang_app::worker::{load_dictionaries, KeyWorker};
use rightlang_core::layout::{KeyTranslator, KeyboardLayout};
use rightlang_macos::hook::{is_accessibility_trusted, EventTapListener};
use rightlang_macos::tis::InputSourceManager;

/// Commands sent from the menubar tray or UI thread to the keyboard worker thread.
enum WorkerCommand {
    SetMode(CorrectionMode),
}

struct TrayTimerState {
    tray_manager: TrayManager,
    cmd_tx: crossbeam_channel::Sender<WorkerCommand>,
    running: Arc<AtomicBool>,
    listener_weak: std::sync::Weak<EventTapListener>,
}

extern "C" fn tray_timer_callback(_timer: CFRunLoopTimerRef, info: *mut c_void) {
    if info.is_null() {
        return;
    }
    let state = unsafe { &mut *(info as *mut TrayTimerState) };
    while let Some(action) = state.tray_manager.check_action() {
        match action {
            TrayAction::SetMode(mode) => {
                state.tray_manager.set_mode(mode);
                let _ = state.cmd_tx.send(WorkerCommand::SetMode(mode));
                println!("[RightLang] Tray menu set mode to: {:?}", mode);
            }
            TrayAction::ToggleStartAtLogin => {
                match rightlang_app::service::toggle_launchd_service() {
                    Ok(new_state) => {
                        state.tray_manager.set_start_at_login(new_state);
                        println!("[RightLang] Start at login toggled to: {}", new_state);
                    }
                    Err(err) => {
                        eprintln!("[RightLang] Failed to toggle start at login: {}", err);
                        state
                            .tray_manager
                            .set_start_at_login(rightlang_app::service::is_service_installed());
                    }
                }
            }
            TrayAction::OpenConfigDir => {
                let _ = TrayManager::open_config_dir();
            }
            TrayAction::Quit => {
                println!("\nShutdown requested from menubar tray. Stopping RightLang-RS daemon...");
                state.running.store(false, Ordering::SeqCst);
                if let Some(l) = state.listener_weak.upgrade() {
                    l.stop();
                }
            }
        }
    }
}

fn print_help() {
    println!(
        r#"RightLang-RS Daemon {}
Automatic Thai <-> English keyboard layout corrector for macOS

USAGE:
    rightlang-daemon [OPTIONS]

OPTIONS:
    -h, --help               Print this help text and exit
    -v, --version            Print version information and exit
    --convert <TEXT>         Convert string directly between layouts and exit
    --check <TEXT>           Check language and dictionary membership and exit
    --status                 Display accessibility, input source, and daemon diagnostics
    --install-service        Install and load launchd agent (~/Library/LaunchAgents)
    --uninstall-service      Unload and remove launchd agent
"#,
        env!("CARGO_PKG_VERSION")
    );
}

fn print_status() {
    println!("=== RightLang-RS Diagnostics & Status ===");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    // 1. Accessibility permissions
    let access = is_accessibility_trusted(false);
    println!("\n[Accessibility Permissions]");
    if access {
        println!("  Status: GRANTED (EventTap is enabled to intercept keys)");
    } else {
        println!("  Status: NOT GRANTED");
        println!("  Please enable in: System Settings -> Privacy & Security -> Accessibility");
    }

    // 2. Active Input Source
    println!("\n[Input Sources (TIS)]");
    let tis = InputSourceManager::new();
    match tis.get_current_source_id() {
        Ok(id) => println!("  Active Source ID: {}", id),
        Err(err) => println!("  Active Source: <Unavailable: {}>", err),
    }

    match tis.list_keyboard_input_sources() {
        Ok(sources) => {
            println!("  Enabled Keyboard Sources ({} found):", sources.len());
            for src in sources {
                let is_th = src.languages.iter().any(|l| l.starts_with("th"));
                let is_en = src.languages.iter().any(|l| l.starts_with("en"));
                let flag = if is_th {
                    "[TH]"
                } else if is_en {
                    "[EN]"
                } else {
                    "    "
                };
                println!("    {} {} ({})", flag, src.name, src.id);
            }
        }
        Err(err) => {
            println!("  Failed to list input sources: {}", err);
        }
    }

    // 3. Dictionaries & Word Counts
    println!("\n[Dictionaries]");
    let dict = load_dictionaries();
    println!("  Standard Thai words:    {}", dict.thai_trie().len());
    println!("  Standard English words: {}", dict.eng_trie().len());
    println!("  Custom User Thai words: {}", dict.user_thai_words().len());
    println!("  Custom User Eng words:  {}", dict.user_eng_words().len());

    // 4. Configuration & Paths
    println!("\n[Paths & Configuration]");
    let cfg_path = AppConfig::default_config_path();
    let th_user = AppConfig::default_user_thai_dict_path();
    let en_user = AppConfig::default_user_eng_dict_path();
    println!(
        "  Config File:       {:?} (exists: {})",
        cfg_path,
        cfg_path.exists()
    );
    println!(
        "  User Thai Dict:    {:?} (exists: {})",
        th_user,
        th_user.exists()
    );
    println!(
        "  User English Dict: {:?} (exists: {})",
        en_user,
        en_user.exists()
    );

    // 5. LaunchAgent Service
    println!("\n[LaunchAgent Service]");
    let is_installed = rightlang_app::service::is_service_installed();
    let plist_path = rightlang_app::service::launchd_plist_path();
    println!(
        "  Service Plist:     {:?} (installed: {})",
        plist_path, is_installed
    );
    println!("=========================================");
}

fn install_launchd_service() {
    match rightlang_app::service::install_launchd_service() {
        Ok(plist_path) => {
            println!("Successfully installed and loaded RightLang LaunchAgent:");
            println!("  Plist:  {:?}", plist_path);
            println!("RightLang is now running and will auto-start upon login!");
        }
        Err(err) => {
            eprintln!("Error installing LaunchAgent: {}", err);
            std::process::exit(1);
        }
    }
}

fn uninstall_launchd_service() {
    match rightlang_app::service::uninstall_launchd_service() {
        Ok(()) => {
            println!("Successfully unloaded and removed RightLang LaunchAgent service.");
        }
        Err(err) => {
            eprintln!("Error uninstalling LaunchAgent: {}", err);
            std::process::exit(1);
        }
    }
}

fn handle_cli_args(args: &[String]) {
    if args.len() <= 1 {
        return;
    }

    match args[1].as_str() {
        "-h" | "--help" => {
            print_help();
            std::process::exit(0);
        }
        "-v" | "--version" => {
            println!("rightlang-daemon {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        "--convert" => {
            if args.len() < 3 {
                eprintln!(
                    "Error: --convert requires a text argument.\nUsage: rightlang-daemon --convert <TEXT>"
                );
                std::process::exit(1);
            }
            let text = &args[2];
            let translator = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);
            let converted = translator.convert_str(text);
            println!("{}", converted);
            std::process::exit(0);
        }
        "--check" => {
            if args.len() < 3 {
                eprintln!(
                    "Error: --check requires a text argument.\nUsage: rightlang-daemon --check <TEXT>"
                );
                std::process::exit(1);
            }
            let text = &args[2];
            let dict = load_dictionaries();
            let translator = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);

            let lang = dict.detect_language(text);
            let in_th = dict.contains_thai(text);
            let in_en = dict.contains_english(text);
            let user_th = dict.user_thai_words().contains(text);
            let user_en = dict.user_eng_words().contains(&text.to_lowercase());
            let converted = translator.convert_str(text);
            let conv_lang = dict.detect_language(&converted);
            let mismatch = dict.detect_mismatched_language(text, &translator);

            println!("Check Results for \"{}\":", text);
            println!("  Detected Language:       {:?}", lang);
            println!("  In Thai Dictionary:      {} (User custom: {})", in_th, user_th);
            println!("  In English Dictionary:   {} (User custom: {})", in_en, user_en);
            println!("  Converted Layout:        \"{}\"", converted);
            println!("  Converted Language:      {:?}", conv_lang);
            println!("  Language Mismatch:       {:?}", mismatch);
            if let Some(target) = mismatch {
                println!("  -> Suggested Correction: \"{}\" ({:?})", converted, target);
            } else if in_th || in_en {
                println!(
                    "  -> Text is already a valid dictionary word in {:?}",
                    lang.unwrap()
                );
            } else {
                println!("  -> Word not recognized in dictionary");
            }
            std::process::exit(0);
        }
        "--status" => {
            print_status();
            std::process::exit(0);
        }
        "--install-service" => {
            install_launchd_service();
            std::process::exit(0);
        }
        "--uninstall-service" => {
            uninstall_launchd_service();
            std::process::exit(0);
        }
        unknown => {
            eprintln!(
                "Unknown option: {}\nRun 'rightlang-daemon --help' for available options.",
                unknown
            );
            std::process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    handle_cli_args(&args);

    println!("Starting RightLang-RS Daemon...");

    // Configure macOS application dock policy (Accessory / Menubar-only)
    TrayManager::setup_dock_policy();

    // Check accessibility permissions
    if !is_accessibility_trusted(true) {
        eprintln!(
            "Warning: RightLang-RS requires Accessibility permissions to intercept keystrokes.\n\
             Please grant permissions in System Settings -> Privacy & Security -> Accessibility."
        );
    }

    // Load configuration
    let config = AppConfig::load_or_default(AppConfig::default_config_path());
    println!(
        "Configuration loaded: default mode={:?}, key_delay={}us, notify={}",
        config.mode, config.key_delay_us, config.notify_on_correction
    );

    // Initialize notification manager
    let notify_manager = Arc::new(NotificationManager::new(config.notify_on_correction));

    // Load dictionaries
    let dict = load_dictionaries();
    println!(
        "Dictionaries loaded: Thai words={}, English words={}",
        dict.thai_trie().len(),
        dict.eng_trie().len()
    );

    // Channels for keyboard events and worker commands
    let (tx, rx) = unbounded();
    let (cmd_tx, cmd_rx) = unbounded();

    // Create KeyWorker
    let mut worker = KeyWorker::new(config.clone(), dict);

    // Spawn dedicated worker thread
    let running = Arc::new(AtomicBool::new(true));
    let running_worker = Arc::clone(&running);
    let notify_mgr_worker = Arc::clone(&notify_manager);
    let notify_on_correction = config.notify_on_correction;

    let worker_handle = thread::spawn(move || {
        while running_worker.load(Ordering::SeqCst) {
            crossbeam_channel::select! {
                recv(rx) -> event_res => {
                    match event_res {
                        Ok(event) => {
                            if let Some(record) = worker.handle_event(&event) {
                                println!(
                                    "[RightLang] Corrected '{}' -> '{}' to {:?}",
                                    record.original, record.replacement, record.target_language
                                );
                                if notify_on_correction {
                                    notify_mgr_worker.notify_correction(&record.original, &record.replacement);
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
                recv(cmd_rx) -> cmd_res => {
                    match cmd_res {
                        Ok(WorkerCommand::SetMode(mode)) => {
                            worker.set_mode(mode);
                            println!("[RightLang] Worker updated correction mode: {:?}", mode);
                        }
                        Err(_) => break,
                    }
                }
                default(Duration::from_millis(200)) => continue,
            }
        }
    });

    // Create EventTapListener
    let listener = match EventTapListener::new(tx) {
        Ok(l) => Arc::new(l),
        Err(err) => {
            eprintln!("Failed to initialize macOS EventTapListener: {}", err);
            running.store(false, Ordering::SeqCst);
            let _ = worker_handle.join();
            std::process::exit(1);
        }
    };

    // Set up Ctrl+C signal handler for graceful shutdown
    let listener_weak = Arc::downgrade(&listener);
    let running_signal = Arc::clone(&running);
    if let Err(err) = ctrlc::set_handler(move || {
        println!("\nShutdown signal received. Stopping RightLang-RS daemon...");
        running_signal.store(false, Ordering::SeqCst);
        if let Some(l) = listener_weak.upgrade() {
            l.stop();
        }
    }) {
        eprintln!("Warning: Failed to set Ctrl-C handler: {}", err);
    }

    // Initialize Status Bar Menubar Tray
    let tray_manager = match TrayManager::new(config.mode) {
        Ok(tm) => Some(tm),
        Err(err) => {
            eprintln!("Warning: Menubar tray initialization failed: {}", err);
            None
        }
    };

    // Set up CFRunLoopTimer on the main thread to poll tray actions
    let mut timer_ref: Option<CFRunLoopTimerRef> = None;
    let mut timer_state_box: Option<Box<TrayTimerState>> = None;

    if let Some(tm) = tray_manager {
        let mut state = Box::new(TrayTimerState {
            tray_manager: tm,
            cmd_tx,
            running: Arc::clone(&running),
            listener_weak: Arc::downgrade(&listener),
        });

        let state_ptr = state.as_mut() as *mut TrayTimerState as *mut c_void;
        timer_state_box = Some(state);

        let mut timer_context = CFRunLoopTimerContext {
            version: 0,
            info: state_ptr,
            retain: None,
            release: None,
            copyDescription: None,
        };

        unsafe {
            let run_loop = CFRunLoopGetCurrent();
            let timer = CFRunLoopTimerCreate(
                kCFAllocatorDefault,
                CFAbsoluteTimeGetCurrent() + 0.1,
                0.1,
                0,
                0,
                tray_timer_callback,
                &mut timer_context,
            );

            if !timer.is_null() {
                CFRunLoopAddTimer(run_loop, timer, kCFRunLoopCommonModes);
                timer_ref = Some(timer);
            }
        }
    }

    println!("RightLang-RS daemon running. Press Ctrl+C or use the Menubar tray to stop.");

    // Run the event tap loop (blocking on CFRunLoop)
    if let Err(err) = listener.run() {
        eprintln!("EventTap run loop error: {}", err);
    }

    // Cleanup tray timer
    if let Some(timer) = timer_ref {
        unsafe {
            let run_loop = CFRunLoopGetCurrent();
            CFRunLoopTimerInvalidate(timer);
            CFRunLoopRemoveTimer(run_loop, timer, kCFRunLoopCommonModes);
            CFRelease(timer as _);
        }
    }
    drop(timer_state_box);

    // Cleanup and shutdown
    running.store(false, Ordering::SeqCst);
    listener.stop();
    drop(listener);

    let _ = worker_handle.join();
    println!("RightLang-RS daemon stopped cleanly.");
}
