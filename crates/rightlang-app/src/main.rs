use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use core_foundation_sys::base::{kCFAllocatorDefault, CFRelease};
use core_foundation_sys::date::CFAbsoluteTimeGetCurrent;
use core_foundation_sys::runloop::{
    kCFRunLoopCommonModes, CFRunLoopAddTimer, CFRunLoopGetCurrent, CFRunLoopRemoveSource,
    CFRunLoopRemoveTimer, CFRunLoopTimerContext, CFRunLoopTimerCreate, CFRunLoopTimerInvalidate,
    CFRunLoopTimerRef,
};
use crossbeam_channel::unbounded;
use rightlang_app::config::{AppConfig, CorrectionMode};
use rightlang_app::notify::NotificationManager;
use rightlang_app::tray::{TrayAction, TrayManager};
use rightlang_app::ui::DashboardUI;
use rightlang_app::worker::{load_dictionaries, KeyWorker};
use rightlang_core::layout::{KeyTranslator, KeyboardLayout};
use rightlang_core::DictionaryFilter;
use rightlang_macos::event::KeyEvent;
use rightlang_macos::hook::{is_accessibility_trusted, EventTapListener};
use rightlang_macos::tis::InputSourceManager;

/// Commands sent from the menubar tray or UI thread to the keyboard worker thread.
enum WorkerCommand {
    SetMode(CorrectionMode),
}

static CMD_TX: Mutex<Option<crossbeam_channel::Sender<WorkerCommand>>> = Mutex::new(None);
static DICT_REF: Mutex<Option<Arc<DictionaryFilter>>> = Mutex::new(None);
static PENDING_UI_MODE: AtomicI32 = AtomicI32::new(-1);

extern "C" fn on_playground_input(input_ptr: *const std::os::raw::c_char) {
    if input_ptr.is_null() {
        return;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(input_ptr) };
    let input = c_str.to_string_lossy();
    if input.trim().is_empty() {
        DashboardUI::set_playground_result("(รอรับข้อความทดสอบ)");
        return;
    }

    let translator = KeyTranslator::new(KeyboardLayout::Qwerty, KeyboardLayout::Kedmanee);
    let converted = translator.convert_str(&input);

    if let Ok(lock) = DICT_REF.lock() {
        if let Some(ref dict) = *lock {
            if let Some(target) = dict.detect_mismatched_language(&input, &translator) {
                let res = format!("{}  ➔  {} (ตรวจพบภาษา {:?})", input, converted, target);
                DashboardUI::set_playground_result(&res);
                return;
            } else if let Some(lang) = dict.detect_language(&input) {
                let res = format!("{} (ตรงกับคำในพจนานุกรม {:?})", input, lang);
                DashboardUI::set_playground_result(&res);
                return;
            }
        }
    }

    let res = format!("{}  ➔  {}", input, converted);
    DashboardUI::set_playground_result(&res);
}

extern "C" fn on_mode_change(mode_idx: i32) {
    let mode = match mode_idx {
        0 => CorrectionMode::Auto,
        1 => CorrectionMode::Manual,
        _ => CorrectionMode::Disabled,
    };
    PENDING_UI_MODE.store(mode_idx, Ordering::SeqCst);
    if let Ok(lock) = CMD_TX.lock() {
        if let Some(ref tx) = *lock {
            let _ = tx.send(WorkerCommand::SetMode(mode));
            println!("[Phasa] Dashboard changed correction mode to: {:?}", mode);
        }
    }
}

extern "C" fn on_autostart_toggle(_enabled: bool) {
    let _ = rightlang_app::service::toggle_launchd_service();
}

extern "C" fn on_notify_toggle(_enabled: bool) {
    // In-memory toggle
}

extern "C" fn on_open_config() {
    let _ = TrayManager::open_config_dir();
}

extern "C" fn on_open_accessibility() {
    let _ = TrayManager::open_accessibility_settings();
}

struct TrayTimerState {
    tray_manager: TrayManager,
    cmd_tx: crossbeam_channel::Sender<WorkerCommand>,
    running: Arc<AtomicBool>,
    event_tx: crossbeam_channel::Sender<KeyEvent>,
    listener: Arc<Mutex<Option<Arc<EventTapListener>>>>,
    source_ptr: Arc<Mutex<Option<*mut c_void>>>,
}

unsafe impl Send for TrayTimerState {}
unsafe impl Sync for TrayTimerState {}

extern "C" fn tray_timer_callback(_timer: CFRunLoopTimerRef, info: *mut c_void) {
    if info.is_null() {
        return;
    }
    let state = unsafe { &mut *(info as *mut TrayTimerState) };

    // Synchronize mode change triggered from Dashboard UI
    let pending_mode = PENDING_UI_MODE.swap(-1, Ordering::SeqCst);
    if pending_mode >= 0 {
        let mode = match pending_mode {
            0 => CorrectionMode::Auto,
            1 => CorrectionMode::Manual,
            _ => CorrectionMode::Disabled,
        };
        state.tray_manager.set_mode(mode);
        println!("[Phasa] Synced tray mode from Dashboard to: {:?}", mode);
    }

    while let Some(action) = state.tray_manager.check_action() {
        match action {
            TrayAction::OpenDashboard => {
                DashboardUI::show();
            }
            TrayAction::OpenAccessibility => {
                let _ = TrayManager::open_accessibility_settings();
            }
            TrayAction::SetMode(mode) => {
                state.tray_manager.set_mode(mode);
                DashboardUI::set_mode(mode);
                let _ = state.cmd_tx.send(WorkerCommand::SetMode(mode));
                println!("[Phasa] Tray menu set mode to: {:?}", mode);
            }
            TrayAction::ToggleStartAtLogin => {
                match rightlang_app::service::toggle_launchd_service() {
                    Ok(new_state) => {
                        state.tray_manager.set_start_at_login(new_state);
                        DashboardUI::set_autostart(new_state);
                        println!("[Phasa] Start at login toggled to: {}", new_state);
                    }
                    Err(err) => {
                        eprintln!("[Phasa] Failed to toggle start at login: {}", err);
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
                println!("\nShutdown requested from menubar tray. Stopping Phasa daemon...");
                state.running.store(false, Ordering::SeqCst);
                if let Ok(lock) = state.listener.lock() {
                    if let Some(l) = lock.as_ref() {
                        l.stop();
                    }
                }
                DashboardUI::stop();
            }
        }
    }

    // Check accessibility status dynamically
    let trusted = is_accessibility_trusted(false);
    DashboardUI::set_accessibility_status(trusted);

    let needs_attach = state.listener.lock().map(|l| l.is_none()).unwrap_or(false);
    if needs_attach {
        if trusted {
            if let Ok(l) = EventTapListener::new(state.event_tx.clone()) {
                let run_loop = unsafe { CFRunLoopGetCurrent() };
                if let Ok(source) = l.start_with_run_loop(run_loop as _) {
                    let arc_l = Arc::new(l);
                    if let Ok(mut lock) = state.listener.lock() {
                        *lock = Some(arc_l);
                    }
                    if let Ok(mut src_lock) = state.source_ptr.lock() {
                        *src_lock = Some(source as _);
                    }
                    state.tray_manager.set_accessibility_status(true);
                    state.tray_manager.set_mode(state.tray_manager.current_mode());
                    println!("[Phasa] Accessibility granted! Event tap attached and active.");
                }
            }
        } else {
            state.tray_manager.set_accessibility_status(false);
        }
    }
}

fn print_help() {
    println!(
        r#"Phasa (ภาษา) v{}
Zero-latency intelligent Thai <-> English keyboard layout corrector for macOS

USAGE:
    phasa [OPTIONS]

OPTIONS:
    -h, --help               Print this help text and exit
    -v, --version            Print version information and exit
    --dashboard              Open the Phasa GUI Dashboard & Settings Window
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
    println!("=== Phasa (ภาษา) Diagnostics & Status ===");
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
            println!("Successfully installed and loaded Phasa LaunchAgent:");
            println!("  Plist:  {:?}", plist_path);
            println!("Phasa is now running and will auto-start upon login!");
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
            println!("Successfully unloaded and removed Phasa LaunchAgent service.");
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
            println!("phasa {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        "--dashboard" => {
            // Handled in main()
        }
        "--convert" => {
            if args.len() < 3 {
                eprintln!(
                    "Error: --convert requires a text argument.\nUsage: phasa --convert <TEXT>"
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
                    "Error: --check requires a text argument.\nUsage: phasa --check <TEXT>"
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
                "Unknown option: {}\nRun 'phasa --help' for available options.",
                unknown
            );
            std::process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    handle_cli_args(&args);

    println!("Starting Phasa (ภาษา) Daemon...");

    // Check accessibility permissions
    let initial_trusted = is_accessibility_trusted(false);
    if !initial_trusted {
        eprintln!(
            "Warning: Phasa requires Accessibility permissions to intercept keystrokes.\n\
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
    let dict = Arc::new(load_dictionaries());
    println!(
        "Dictionaries loaded: Thai words={}, English words={}",
        dict.thai_trie().len(),
        dict.eng_trie().len()
    );

    // Save global dictionary ref for live playground
    if let Ok(mut lock) = DICT_REF.lock() {
        *lock = Some(Arc::clone(&dict));
    }

    // Channels for keyboard events and worker commands
    let (tx, rx) = unbounded();
    let (cmd_tx, cmd_rx) = unbounded();

    // Save global command sender
    if let Ok(mut lock) = CMD_TX.lock() {
        *lock = Some(cmd_tx.clone());
    }

    // Create KeyWorker
    let mut worker = KeyWorker::new(config.clone(), (*dict).clone());

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
                                    "[Phasa] Corrected '{}' -> '{}' to {:?}",
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
                            println!("[Phasa] Worker updated correction mode: {:?}", mode);
                        }
                        Err(_) => break,
                    }
                }
                default(Duration::from_millis(200)) => continue,
            }
        }
    });

    let run_loop = unsafe { CFRunLoopGetCurrent() };

    // Register GUI Dashboard Callbacks & Initialize Window
    DashboardUI::register_callbacks(
        on_playground_input,
        on_mode_change,
        on_autostart_toggle,
        on_notify_toggle,
        on_open_config,
        on_open_accessibility,
    );
    DashboardUI::init(
        config.mode,
        rightlang_app::service::is_service_installed(),
        config.notify_on_correction,
    );
    DashboardUI::set_accessibility_status(initial_trusted);

    // Initialize Status Bar Menubar Tray
    let mut tray_manager = match TrayManager::new(config.mode) {
        Ok(tm) => Some(tm),
        Err(err) => {
            eprintln!("Warning: Menubar tray initialization failed: {}", err);
            None
        }
    };

    if let Some(ref mut tm) = tray_manager {
        tm.set_accessibility_status(initial_trusted);
    }

    // Try to attach EventTapListener if already trusted
    let mut initial_listener: Option<Arc<EventTapListener>> = None;
    let mut initial_source: Option<*mut c_void> = None;

    if initial_trusted {
        match EventTapListener::new(tx.clone()) {
            Ok(l) => match l.start_with_run_loop(run_loop as _) {
                Ok(source) => {
                    initial_listener = Some(Arc::new(l));
                    initial_source = Some(source as _);
                    println!("[Phasa] Keyboard event tap successfully hooked on main run loop.");
                }
                Err(err) => eprintln!("Warning: Failed to attach event tap: {}", err),
            },
            Err(err) => eprintln!("Warning: Failed to create EventTapListener: {}", err),
        }
    } else {
        println!("[Phasa] Waiting for Accessibility permission. Showing Dashboard Window...");
    }

    // Show Dashboard Window on launch so the user can see status & live playground
    DashboardUI::show();

    let listener_arc = Arc::new(Mutex::new(initial_listener));
    let source_ptr_arc = Arc::new(Mutex::new(initial_source));

    // Set up Ctrl+C signal handler for graceful shutdown
    let listener_signal = Arc::clone(&listener_arc);
    let running_signal = Arc::clone(&running);
    if let Err(err) = ctrlc::set_handler(move || {
        println!("\nShutdown signal received. Stopping Phasa daemon...");
        running_signal.store(false, Ordering::SeqCst);
        if let Ok(lock) = listener_signal.lock() {
            if let Some(l) = lock.as_ref() {
                l.stop();
            }
        }
        DashboardUI::stop();
    }) {
        eprintln!("Warning: Failed to set Ctrl-C handler: {}", err);
    }

    // Set up CFRunLoopTimer on the main thread to poll tray actions and permissions
    let mut timer_ref: Option<CFRunLoopTimerRef> = None;
    let mut timer_state_box: Option<Box<TrayTimerState>> = None;

    if let Some(tm) = tray_manager {
        let mut state = Box::new(TrayTimerState {
            tray_manager: tm,
            cmd_tx,
            running: Arc::clone(&running),
            event_tx: tx,
            listener: Arc::clone(&listener_arc),
            source_ptr: Arc::clone(&source_ptr_arc),
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
            let timer = CFRunLoopTimerCreate(
                kCFAllocatorDefault,
                CFAbsoluteTimeGetCurrent() + 0.1,
                0.2,
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

    println!("Phasa (ภาษา) running with Dashboard. Press Ctrl+C or use the Menubar tray to stop.");

    // Run the native AppKit main event loop (keeps app alive indefinitely)
    DashboardUI::run();

    // Cleanup tray timer
    if let Some(timer) = timer_ref {
        unsafe {
            CFRunLoopTimerInvalidate(timer);
            CFRunLoopRemoveTimer(run_loop, timer, kCFRunLoopCommonModes);
            CFRelease(timer as _);
        }
    }

    // Cleanup event tap run loop source
    if let Ok(lock) = source_ptr_arc.lock() {
        if let Some(source) = *lock {
            unsafe {
                CFRunLoopRemoveSource(run_loop, source as _, kCFRunLoopCommonModes);
                CFRelease(source as _);
            }
        }
    }

    // Ensure worker finishes
    let _ = worker_handle.join();
    drop(timer_state_box);
    println!("Phasa daemon successfully terminated.");
}
