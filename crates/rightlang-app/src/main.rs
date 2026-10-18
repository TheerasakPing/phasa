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
use rightlang_macos::hook::{is_accessibility_trusted, EventTapListener};

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

fn main() {
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
