# RightLang Rust Rewrite ("RightLang-RS") Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a blazing-fast, lightweight, memory-efficient macOS background utility in Rust to replace RightLang, automatically detecting and correcting language mismatches (Thai ↔ English) with sub-millisecond latency and modern macOS integration.

**Architecture:** A multi-threaded, channel-based daemon architecture in Rust:
1. **Core Translation Engine (`rightlang-core`)**: Pure Rust crate containing bi-directional layout maps (Kedmanee, Pattachote, QWERTY, Dvorak), compact Trie/FST dictionary matching, and fuzzy key distance evaluation.
2. **Platform Hook & Dispatcher (`rightlang-macos`)**: Low-level macOS integration via `CGEventTap`, Carbon Text Input Source (`TIS`) switching, and synthetic event replay (`CGEventPost`).
3. **Application & Tray Controller (`rightlang-app`)**: Menu-bar status item, configuration management (TOML), per-application rules, and notification dispatch.

**Tech Stack:**
- Language: Rust 2021 Edition
- Dictionary Engine: `fst` or compact Double-Array Trie (`daachorse`)
- Concurrency: `crossbeam-channel` or `flume` (zero-allocation lock-free queue)
- macOS FFI: `core-foundation`, `core-graphics`, `carbon-sys` / custom FFI bindings for `TIS`
- Configuration: `serde`, `toml`
- UI / Tray: `tray-icon` + `muda` or native minimal AppKit status item

---

## Global Constraints

- **Platform**: macOS 12.0+ (Apple Silicon `aarch64-apple-darwin` + Intel `x86_64-apple-darwin`).
- **Memory Ceiling**: Maximum RSS < 20 MB during full dictionary operations (RightLang was ~60-80 MB).
- **Latency Budget**: Key-to-decision time < 1 ms on a 30-character buffer.
- **Safety**: 100% pure safe Rust for `rightlang-core`; all `unsafe` blocks isolated and documented in `rightlang-macos`.
- **Zero Panics**: Event hook loop must never panic or crash the daemon; timeouts handled gracefully.

## Review Focus

1. **Slow Electron Apps**: Posting synthetic backspaces too fast to Chrome/Slack/VS Code causes dropped keystrokes (requires microsecond debounce / batching).
2. **Event Tap Loopbacks**: Injected synthetic keys must never re-trigger the input hook (tagging with `kCGEventSourceUserData`).
3. **Event Tap Timeout Recovery**: macOS automatically disables event taps if they take > 2 seconds or during sleep/wake (must auto-re-enable).
4. **Compound Thai Vowels/Tonal Marks**: Deleting Thai characters containing upper/lower vowel combinations (e.g., `น้ำ`, `เป็น`) requires accurate UTF-16 backspace counts.
5. **App Whitelist/Blacklist**: Terminals, password fields, and games must be excluded cleanly by checking frontmost application bundle ID.

---

### Task 1: Scaffolding Workspace & Core Keyboard Layout Mapping

**Files:**
- Create: `Cargo.toml` (Workspace root)
- Create: `crates/rightlang-core/Cargo.toml`
- Create: `crates/rightlang-core/src/lib.rs`
- Create: `crates/rightlang-core/src/layout.rs`
- Test: `crates/rightlang-core/tests/test_layout.rs`

**Interfaces:**
- Produces:
  ```rust
  pub enum KeyboardLayout { Kedmanee, Pattachote, Qwerty, Dvorak }
  pub struct KeyTranslator { ... }
  impl KeyTranslator {\n      pub fn new(source: KeyboardLayout, target: KeyboardLayout) -> Self;
      pub fn convert_char(&self, ch: char) -> char;
      pub fn convert_str(&self, input: &str) -> String;
  }
  ```

- [x] **Step 1: Write the failing unit tests for layout conversion**
  - Test converting English `8iy[` to Thai `ครับ`.
  - Test converting Thai `สวัสดี` to English `l;ylfu`.
  - Test shift key characters (e.g. `!` ↔ `+`, `@` ↔ `๑`).

- [x] **Step 2: Run test to verify it fails**
  `cargo test -p rightlang-core --test test_layout`

- [x] **Step 3: Implement minimal KeyTranslator mapping**
  - Implement bidirectional static array/hash map lookup using the exact Kedmanee/QWERTY tables reversed from RightLang.

- [x] **Step 4: Run test to verify it passes**
  `cargo test -p rightlang-core --test test_layout`

- [x] **Step 5: Commit**
  `git commit -m "feat(core): implement bidirectional keyboard layout translator"`

---

### Task 2: High-Performance Compact Trie Dictionary & Word Detection

**Files:**
- Create: `crates/rightlang-core/src/dictionary.rs`
- Create: `crates/rightlang-core/src/trie.rs`
- Test: `crates/rightlang-core/tests/test_dictionary.rs`
- Resource: Import `thai.txt`, `eng.txt` from extracted RightLang assets.

**Interfaces:**
- Produces:
  ```rust
  pub struct DictionaryFilter { ... }
  impl DictionaryFilter {
      pub fn from_wordlist_str(words: &str) -> Self;
      pub fn contains(&self, word: &str) -> bool;
      pub fn is_prefix(&self, prefix: &str) -> bool;
      pub fn detect_language(&self, buffer: &str) -> Option<Language>;
  }
  ```

- [x] **Step 1: Write unit tests for Trie dictionary lookups and language detection**
  - Test known Thai words (`สวัสดี`, `เป็น`, `ครับ`).
  - Test known English words (`hello`, `test`, `vscode`).
  - Test partial prefix detection.

- [x] **Step 2: Run test to verify failure**
  `cargo test -p rightlang-core --test test_dictionary`

- [x] **Step 3: Implement compact Trie data structure**
  - Implement Double-Array Trie or memory-efficient Node-based Trie in Rust.
  - Load Thai and English dictionaries into separate instances.

- [x] **Step 4: Run tests and benchmarks**
  `cargo test -p rightlang-core --test test_dictionary`

- [x] **Step 5: Commit**
  `git commit -m "feat(core): implement compact dictionary trie and language detection"`

---

### Task 3: Physical Keyboard Adjacency Matrix & Typo Distance

**Files:**
- Create: `crates/rightlang-core/src/distance.rs`
- Test: `crates/rightlang-core/tests/test_distance.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct KeyDistanceMatrix { ... }
  impl KeyDistanceMatrix {
      pub fn from_adjacent_pairs(pairs: &str) -> Self;
      pub fn distance(&self, a: char, b: char) -> u32;
      pub fn fuzzy_match(&self, input: &str, candidate: &str, max_distance: u32) -> bool;
  }
  ```

- [x] **Step 1: Write unit test for adjacent physical keys**
  - Test distance between 'q' and 'w' is 1, between 'q' and 'p' > 1.

- [x] **Step 2: Run test to verify failure**
  `cargo test -p rightlang-core --test test_distance`

- [x] **Step 3: Implement distance matrix parser from `adjacent-column.txt`**
  - Graph/table-based distance evaluator for physical key proximity.

- [x] **Step 4: Run test to verify it passes**
  `cargo test -p rightlang-core --test test_distance`

- [x] **Step 5: Commit**
  `git commit -m "feat(core): implement physical key distance matrix"`

---

### Task 4: macOS Event Tap Hook & Event Filtering (`rightlang-macos`)\n
**Files:**
- Create: `crates/rightlang-macos/Cargo.toml`
- Create: `crates/rightlang-macos/src/lib.rs`
- Create: `crates/rightlang-macos/src/hook.rs`
- Create: `crates/rightlang-macos/src/event.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct EventTapListener { ... }
  impl EventTapListener {
      pub fn new(event_sender: crossbeam_channel::Sender<KeyEvent>) -> Result<Self, HookError>;
      pub fn run_loop(&self);
      pub fn stop(&self);
  }
  ```

- [x] **Step 1: Implement FFI bindings for `CGEventTapCreate`, `CFMachPort`, `CFRunLoop`**
- [x] **Step 2: Implement timeout recovery handler**
  - Intercept `kCGEventTapDisabledByTimeout` and re-enable tap automatically.
- [x] **Step 3: Implement event tagging**
  - Set and check `kCGEventSourceUserData` with unique magic signature to prevent self-hook recursion.
- [x] **Step 4: Verify compiling & link test**
  `cargo check -p rightlang-macos`
- [x] **Step 5: Commit**
  `git commit -m "feat(macos): implement robust CGEventTap keyboard listener"`

---

### Task 5: macOS TIS Keyboard Layout Switching & Synthetic Replay

**Files:**
- Create: `crates/rightlang-macos/src/tis.rs`
- Create: `crates/rightlang-macos/src/dispatcher.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct InputSourceManager { ... }
  impl InputSourceManager {
      pub fn get_current_source_id(&self) -> String;
      pub fn select_input_source(&self, source_id: &str) -> Result<(), TisError>;
  }
  pub struct KeyDispatcher { ... }
  impl KeyDispatcher {
      pub fn replace_text(&self, backspace_count: usize, replacement: &str, delay_us: u64);
  }
  ```

- [x] **Step 1: Implement Carbon `TISCopyCurrentKeyboardInputSource` and `TISSelectInputSource` FFI**
- [x] **Step 2: Implement synthetic backspace generation with `CGEventCreateKeyboardEvent` (KeyCode 0x33)**
- [x] **Step 3: Implement Unicode string injection via `CGEventKeyboardSetUnicodeString`**
- [x] **Step 4: Commit**
  `git commit -m "feat(macos): implement TIS layout switcher and key dispatcher"`

---

### Task 6: Application Rules & Configuration Management

**Files:**
- Create: `crates/rightlang-app/Cargo.toml`
- Create: `crates/rightlang-app/src/config.rs`
- Create: `crates/rightlang-app/src/rules.rs`
- Test: `crates/rightlang-app/tests/test_config.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct AppConfig {
      pub mode: CorrectionMode, // Auto, Manual, Disabled
      pub key_delay_us: u64,
      pub app_rules: HashMap<String, CorrectionMode>,
      pub hotkey: Option<String>,
  }
  ```

- [ ] **Step 1: Write test for TOML configuration parsing and application rules**
- [ ] **Step 2: Implement configuration loader with fallback defaults**
- [ ] **Step 3: Implement bundle ID resolution via `NSWorkspace` / `runningApplications`**
- [ ] **Step 4: Commit**
  `git commit -m "feat(app): implement configuration and application rule manager"`

---

### Task 7: Main Worker Loop & Integration (`rightlang-daemon`)

**Files:**
- Create: `crates/rightlang-app/src/main.rs`
- Create: `crates/rightlang-app/src/worker.rs`

**Interfaces:**
- Wires all crates together:
  `EventTapListener` → `crossbeam_channel` → `KeyWorker` (`keyBuffer` + `TrieFilter` + `KeyTranslator`) → `KeyDispatcher` + `InputSourceManager`.

- [ ] **Step 1: Implement Worker State Machine (Buffering, Matching, Triggering, Undo)**
- [ ] **Step 2: Implement Undo mechanism (restoring previous state if user presses undo shortcut)**
- [ ] **Step 3: End-to-end integration test with mock events**
- [ ] **Step 4: Commit**
  `git commit -m "feat(app): integrate worker loop and end-to-end event pipeline"`

---

### Task 8: Lightweight Status Bar Tray & Notifications

**Files:**
- Create: `crates/rightlang-app/src/tray.rs`
- Create: `crates/rightlang-app/src/notify.rs`

**Interfaces:**
- Native tray icon showing current mode (Auto / Manual / Disabled).
- System notification banner on auto-correction with Undo action.

- [ ] **Step 1: Implement minimal tray menu (`muda` / native AppKit)**
- [ ] **Step 2: Implement notification delivery via macOS `NSUserNotification` or `mac-notification-sys`**
- [ ] **Step 3: Build release binary and verify memory footprint**
- [ ] **Step 4: Commit**
  `git commit -m "feat(app): add menubar tray and native notifications"`
