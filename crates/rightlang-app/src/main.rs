use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::unbounded;
use rightlang_app::config::AppConfig;
use rightlang_app::worker::{load_dictionaries, KeyWorker};
use rightlang_macos::hook::{is_accessibility_trusted, EventTapListener};

fn main() {
    println!("Starting RightLang-RS Daemon...");

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
        "Configuration loaded: default mode={:?}, key_delay={}us",
        config.mode, config.key_delay_us
    );

    // Load dictionaries
    let dict = load_dictionaries();
    println!(
        "Dictionaries loaded: Thai words={}, English words={}",
        dict.thai_trie().len(),
        dict.eng_trie().len()
    );

    // Channel for keyboard events
    let (tx, rx) = unbounded();

    // Create KeyWorker
    let mut worker = KeyWorker::new(config, dict);

    // Spawn dedicated worker thread
    let running = Arc::new(AtomicBool::new(true));
    let running_worker = Arc::clone(&running);

    let worker_handle = thread::spawn(move || {
        while running_worker.load(Ordering::SeqCst) {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(event) => {
                    if let Some(record) = worker.handle_event(&event) {
                        println!(
                            "[RightLang] Corrected '{}' -> '{}' to {:?}",
                            record.original, record.replacement, record.target_language
                        );
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
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

    println!("RightLang-RS daemon running. Press Ctrl+C to stop.");

    // Run the event tap loop (blocking)
    if let Err(err) = listener.run() {
        eprintln!("EventTap run loop error: {}", err);
    }

    // Cleanup and shutdown
    running.store(false, Ordering::SeqCst);
    listener.stop();
    drop(listener);

    let _ = worker_handle.join();
    println!("RightLang-RS daemon stopped cleanly.");
}
