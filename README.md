# Phasa (ภาษา) — ภP Modern Intelligent macOS Input

A blazing-fast, ultra-lightweight macOS background daemon that automatically detects and corrects language mismatches between **Thai (เกษมณี / ปัตตะโชติ)** and **English (QWERTY / Dvorak)** with sub-millisecond latency, native menu bar controls, and modern macOS integration.

Rebranded and rebuilt from scratch in 100% modern Rust.

---

## ⚡ Highlights & Key Improvements

| Metric / Feature | Legacy RightLang (ObjC) | Phasa (ภาษา) (Rust) |
|---|---|---|
| **Binary Size** | ~5 MB bundle + Sparkle + WebKit | **2.3 MB standalone native binary** |
| **Memory Footprint (RSS)** | 60 – 109 MB | **~19 MB** (82% memory reduction) |
| **Lookup Latency** | 50 – 100 μs | **< 1 μs** (Compact Double-Array Trie) |
| **Dictionary Ingestion** | 300 – 500 ms | **< 30 ms** for >38,000 words |
| **IPC & Event Pipeline** | ObjC selector runloop + ARC overhead | **Lock-free zero-allocation channel** (`crossbeam-channel`) |
| **Timeout Recovery** | Dropped on Sleep/Wake timeout | **Automatic self-healing** on `kCGEventTapDisabledByTimeout` |
| **Menubar UI** | Legacy Cocoa nib | **Native lightweight tray** (`tray-icon` + `muda`) |
| **Configuration** | PropertyList in Library | **Human-friendly TOML** (`~/.config/phasa/config.toml`) |

---

## 🏗️ Architecture & Workspace Structure

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
│   │   └── src/event.rs      # KeyEvent model, anti-recursion tagging (0x524C4730)
│   │
│   └── rightlang-app/        # Binary Application & System Integration
│       ├── src/main.rs       # Entrypoint, thread lifecycle & graceful shutdown
│       ├── src/worker.rs     # KeyWorker state machine (Buffer, Match, Replace, Undo)
│       ├── src/config.rs     # TOML configuration manager (~/.config/phasa/config.toml)
│       ├── src/rules.rs      # Frontmost application rule detector & exclusions
│       ├── src/tray.rs       # Menubar status item (tray-icon + muda)
│       └── src/notify.rs     # Native macOS desktop notifications
└── assets/                   # Phasa icon asset (.icns) & Thai/English dictionaries
```

---

## 🚀 Building & Deploying to `/Applications`

### Quick Build & Install
Run the automated packaging script to build the release binary and bundle into `/Applications/Phasa.app`:

```bash
./scripts/package_app.sh --install
```

### Run Directly from Terminal
```bash
# Check status and accessibility permissions
phasa --status

# Test word lookup and suggested conversion
phasa --check "8iy["
# Output:
#   Detected Language:       None
#   Converted Layout:        "ครับ"
#   -> Suggested Correction: "ครับ" (Thai)

# Convert strings on CLI
phasa --convert "g]y'"  # Outputs: เล่น
```

---

## ⚙️ Configuration (`~/.config/phasa/config.toml`)

Phasa automatically creates a default configuration file if one does not exist:

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

- **Undo Last Correction**: `Ctrl + Option + Z` or `Cmd + Shift + Z` (reverts text and layout, auto-learns source term).
- **Convert Selected Text**: `Cmd + /` or `Ctrl + Option + C` (copies highlighted text, converts layout, and pastes).
- **Menubar Controls**: Click the status item in your macOS menu bar (`ภาษา`) to toggle modes (`Auto`, `Manual`, `Disabled`), enable "Start at Login", or open config directory.

---

## 🧪 Testing

Run the full workspace test suite (66 unit and integration tests):
```bash
cargo test --workspace
```
