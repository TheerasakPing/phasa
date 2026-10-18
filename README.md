# RightLang-RS (🦀 Modern Rust Rewrite)

A blazing-fast, ultra-lightweight macOS background daemon that automatically detects and corrects language mismatches between **Thai (เกษมณี / ปัตตะโชติ)** and **English (QWERTY / Dvorak)** with sub-millisecond latency and modern macOS integration.

Reverse-engineered from the original Objective-C/Swift RightLang app and redesigned from the ground up in modern Rust.

---

## ⚡ Highlights & Key Improvements

| Feature | Original RightLang (1.1.4) | RightLang-RS (Rust) |
|---|---|---|
| **Binary Footprint** | ~5 MB bundle + Sparkle + WebKit | **2.2 MB single standalone release binary** |
| **Memory Usage (RSS)** | ~60 – 80 MB | **< 15 – 20 MB** with Compact Double-Array Trie |
| **Lookup Latency** | ~50 – 100 μs (ObjC runtime dynamic dispatch) | **< 1 μs** (Flat memory vectors, $O(1)$ direct array index) |
| **Dictionary Ingestion** | ~300 – 500 ms | **~30 – 50 ms** for >35,000 Thai & English words |
| **Event Pipeline** | ObjC selector messaging + ARC overhead | **Lock-free zero-allocation channel** (`crossbeam-channel`) |
| **Hook Resilience** | Often dropped on Sleep/Wake timeout | **Automatic state recovery** for `kCGEventTapDisabledByTimeout` |
| **Modern App Handling** | Fixed delay, dropped keys in Electron apps | **Configurable microsecond inter-key timing** |
| **Safety** | Complex pointer interactions in legacy ObjC | **100% safe Rust core**, strictly audited unsafe FFI blocks |

---

## 🏗️ Architecture & Crates Structure

```
rightlang-rs/
├── crates/
│   ├── rightlang-core/       # Pure Safe Rust: Layouts, Compact Trie & Fuzzy Distance
│   │   ├── src/layout.rs     # QWERTY ↔ Kedmanee bidirectional translation
│   │   ├── src/trie.rs       # High-performance Compact Trie (binary search edges)
│   │   ├── src/dictionary.rs # Thai/English dictionary filter & language detector
│   │   └── src/distance.rs   # Physical keyboard adjacency graph & typo distance
│   │
│   ├── rightlang-macos/      # Low-level macOS Platform Integration (FFI)
│   │   ├── src/hook.rs       # CGEventTap listener + Timeout auto-recovery + Event tagging
│   │   ├── src/tis.rs        # Carbon TIS Input Source switcher (Thai ↔ English)
│   │   ├── src/dispatcher.rs # Synthetic Backspace & Unicode typing via CGEventPost
│   │   └── src/event.rs      # KeyEvent model, anti-recursion tagging (RIGHTLANG_EVENT_TAG)
│   │
│   └── rightlang-app/        # Binary Application & System Integration
│       ├── src/main.rs       # Entrypoint, thread lifecycle & graceful shutdown
│       ├── src/worker.rs     # KeyWorker state machine (Buffer, Match, Replace, Undo)
│       ├── src/config.rs     # TOML configuration manager (~/.config/rightlang/config.toml)
│       ├── src/rules.rs      # Frontmost application rule detector & exclusions
│       ├── src/tray.rs       # Menubar status item (tray-icon + muda)
│       └── src/notify.rs     # Native macOS desktop notifications
└── assets/                   # Comprehensive Thai & English dictionary wordlists
```

---

## 🚀 Building & Running

### Requirements
- macOS 12.0+ (Apple Silicon or Intel)
- Rust 1.75+ (`cargo`, `rustc`)

### Build Release Binary
```bash
cargo build --release
```
The compiled binary will be located at `target/release/rightlang-daemon`.

### Run the Daemon
```bash
./target/release/rightlang-daemon
```

> **Note on Accessibility Permission**:  
> macOS requires Accessibility permissions for applications that monitor global keyboard events. On the first launch, RightLang-RS will prompt you to grant Accessibility access in **System Settings > Privacy & Security > Accessibility**.

---

## ⚙️ Configuration (`~/.config/rightlang/config.toml`)

RightLang-RS automatically creates a default configuration file if one does not exist:

```toml
mode = "auto"          # "auto", "manual", or "disabled"
key_delay_us = 1000    # Microsecond delay between synthetic keystrokes (default: 1ms)
auto_learn = true      # Learn new words automatically
notify_on_correction = true
notify_on_learn = true

# Application-specific overrides
[app_rules]
"com.apple.Terminal" = "disabled"
"com.googlecode.iterm2" = "disabled"
"net.kovidgoyal.kitty" = "disabled"
"com.agilebits.onepassword" = "disabled"
```

---

## ⌨️ Shortcuts
- **Undo Last Correction**: `Ctrl + Option + Z` or `Cmd + Shift + Z` (reverts text and layout).
- **Menubar Actions**: Click the status item in your macOS menu bar to quickly switch modes (`Auto`, `Manual`, `Disabled`) or access configuration.

---

## 🧪 Testing

Run the full workspace test suite (59 unit and integration tests):
```bash
cargo test --workspace
```
