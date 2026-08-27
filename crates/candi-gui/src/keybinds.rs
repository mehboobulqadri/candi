// SPDX-License-Identifier: AGPL-3.0

//! User-editable keybinds (`keybinds.json` in the XDG config dir): a flat
//! map of action name to key string — `"next_page": ["Right", "PgDn"]` or a
//! bare `"quit": "Q"` — with `"Ctrl+"`/`Shift+`/`Alt+`/`Meta+` modifiers.
//! Reads are tolerant: corrupt entries, unknown actions, and unparsable keys
//! warn and fall back to defaults per entry, never panic. A missing file is
//! seeded with the defaults so the schema is discoverable by hand-editing,
//! following the app-config patterns in [`candi_core::prefs`].

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use eframe::egui;
use egui::Key;

/// An input action dispatchable from [`crate::app::ReaderApp::handle_input`],
/// named as in `keybinds.json` and labeled for the shortcuts window. The set
/// mirrors exactly what was hardcoded there before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    OpenFile,
    SaveState,
    Search,
    ToggleSidebar,
    EditTheme,
    GoToPage,
    KeybindsWindow,
    FocusMode,
    ZoomIn,
    ZoomOut,
    FitWidth,
    PrevPage,
    NextPage,
    CycleTheme,
    Bookmark,
    CloseOverlay,
    Quit,
}

impl Action {
    /// Canonical action order, reused for the JSON defaults, conflict
    /// resolution (later entries win), and shortcuts-window row order.
    const ALL: [Action; 17] = [
        Action::OpenFile,
        Action::SaveState,
        Action::Search,
        Action::ToggleSidebar,
        Action::EditTheme,
        Action::GoToPage,
        Action::KeybindsWindow,
        Action::FocusMode,
        Action::ZoomIn,
        Action::ZoomOut,
        Action::FitWidth,
        Action::PrevPage,
        Action::NextPage,
        Action::CycleTheme,
        Action::Bookmark,
        Action::CloseOverlay,
        Action::Quit,
    ];

    fn name(self) -> &'static str {
        match self {
            Action::OpenFile => "open_file",
            Action::SaveState => "save_state",
            Action::Search => "search",
            Action::ToggleSidebar => "toggle_sidebar",
            Action::EditTheme => "edit_theme",
            Action::GoToPage => "go_to_page",
            Action::KeybindsWindow => "keybinds_window",
            Action::FocusMode => "focus_mode",
            Action::ZoomIn => "zoom_in",
            Action::ZoomOut => "zoom_out",
            Action::FitWidth => "fit_width",
            Action::PrevPage => "prev_page",
            Action::NextPage => "next_page",
            Action::CycleTheme => "cycle_theme",
            Action::Bookmark => "bookmark",
            Action::CloseOverlay => "close_overlay",
            Action::Quit => "quit",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Action::OpenFile => "Open file",
            Action::SaveState => "Save state",
            Action::Search => "Search",
            Action::ToggleSidebar => "Toggle sidebar",
            Action::EditTheme => "Edit theme YAML",
            Action::GoToPage => "Go to page",
            Action::KeybindsWindow => "Keybinds",
            Action::FocusMode => "Focus mode",
            Action::ZoomIn => "Zoom in",
            Action::ZoomOut => "Zoom out",
            Action::FitWidth => "Fit-width zoom",
            Action::PrevPage => "Previous page",
            Action::NextPage => "Next page",
            Action::CycleTheme => "Cycle themes",
            Action::Bookmark => "Bookmark page",
            Action::CloseOverlay => "Close overlay",
            Action::Quit => "Quit",
        }
    }

    fn from_name(name: &str) -> Option<Action> {
        Action::ALL.into_iter().find(|action| action.name() == name)
    }

    /// Default bindings preserving the pre-keybinds hardcoded keys.
    fn defaults(self) -> &'static [&'static str] {
        match self {
            Action::OpenFile => &["Ctrl+O"],
            Action::SaveState => &["Ctrl+S"],
            Action::Search => &["Ctrl+F"],
            Action::ToggleSidebar => &["Ctrl+B"],
            Action::EditTheme => &["Ctrl+E"],
            Action::GoToPage => &["Ctrl+G"],
            Action::KeybindsWindow => &["Ctrl+K"],
            Action::FocusMode => &["F11"],
            Action::CloseOverlay => &["Esc"],
            Action::ZoomIn => &["+", "="],
            Action::ZoomOut => &["-"],
            Action::FitWidth => &["0"],
            Action::CycleTheme => &["T"],
            Action::Bookmark => &["B"],
            Action::Quit => &["Q"],
            Action::PrevPage => &["Left", "PgUp"],
            Action::NextPage => &["Right", "PgDn"],
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A parsed key string plus its modifier requirements; a binding matches an
/// input event when every required bit is held and none extra — so `Ctrl+Q`
/// can never fire a plain `Q` action and vice versa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Binding {
    ctrl: bool,
    shift: bool,
    alt: bool,
    meta: bool,
    key: Key,
}

impl Binding {
    /// Parse `"Ctrl+Shift+S"`-style strings: repeatable `Ctrl/Shift/Alt/Meta`
    /// prefixes, then one key name ([`parse_key`] accepts several spellings,
    /// including bare punctuation like `+`). Case-insensitive throughout.
    fn parse(spec: &str) -> Option<Binding> {
        let lowered = spec.trim().to_ascii_lowercase();
        if lowered.is_empty() {
            return None;
        }
        let mut binding = Binding {
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
            key: Key::Escape,
        };
        // Modifier tokens always end in '+' and are non-empty, so each strip
        // shortens the remainder and the loop terminates.
        let mut rest: &str = &lowered;
        while let Some((bits, tail)) = modifier_prefix(rest) {
            binding.ctrl |= bits[0];
            binding.shift |= bits[1];
            binding.alt |= bits[2];
            binding.meta |= bits[3];
            rest = tail;
        }
        binding.key = parse_key(rest)?;
        Some(binding)
    }

    fn matches(&self, pressed: egui::Modifiers) -> bool {
        self.ctrl == pressed.ctrl
            && self.shift == pressed.shift
            && self.alt == pressed.alt
            && self.meta == (pressed.mac_cmd || pressed.command)
    }

    fn label(self) -> String {
        let mut text = String::new();
        if self.ctrl {
            text.push_str("Ctrl+");
        }
        if self.shift {
            text.push_str("Shift+");
        }
        if self.alt {
            text.push_str("Alt+");
        }
        if self.meta {
            text.push_str("Meta+");
        }
        text.push_str(key_label(self.key));
        text
    }
}

/// Leading `Ctrl+`/`Shift+`/`Alt+`/`Meta+`/`Cmd+` prefix on an already
/// lowercased key string: `(required bits, remainder)`.
fn modifier_prefix(rest: &str) -> Option<([bool; 4], &str)> {
    const PREFIXES: [(&str, [bool; 4]); 5] = [
        ("ctrl+", [true, false, false, false]),
        ("shift+", [false, true, false, false]),
        ("alt+", [false, false, true, false]),
        ("meta+", [false, false, false, true]),
        ("cmd+", [false, false, false, true]),
    ];
    PREFIXES
        .into_iter()
        .find_map(|(prefix, bits)| rest.strip_prefix(prefix).map(|tail| (bits, tail)))
}

const DIGITS: [Key; 10] = [
    Key::Num0,
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
];

const LETTERS: [Key; 26] = [
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
];

const FUNCTION_KEYS: [Key; 12] = [
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
];

/// Accepted key spellings, case-insensitive (`Esc`, `PgDn`, bare `+`, …).
fn parse_key(name: &str) -> Option<Key> {
    let lower = name.to_ascii_lowercase();
    if lower.len() == 1 {
        let byte = lower.as_bytes()[0];
        if let b'0'..=b'9' = byte {
            return DIGITS.get((byte - b'0') as usize).copied();
        }
        if let b'a'..=b'z' = byte {
            return LETTERS.get((byte - b'a') as usize).copied();
        }
    }
    Some(match lower.as_str() {
        "+" | "plus" => Key::Plus,
        "=" | "equals" => Key::Equals,
        "-" | "minus" => Key::Minus,
        "," | "comma" => Key::Comma,
        "." | "period" => Key::Period,
        "/" | "slash" => Key::Slash,
        "escape" | "esc" => Key::Escape,
        "enter" | "return" => Key::Enter,
        "space" => Key::Space,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "insert" => Key::Insert,
        "delete" | "del" => Key::Delete,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" | "pgup" => Key::PageUp,
        "pagedown" | "pgdown" | "pgdn" => Key::PageDown,
        "left" | "arrowleft" => Key::ArrowLeft,
        "right" | "arrowright" => Key::ArrowRight,
        "up" | "arrowup" => Key::ArrowUp,
        "down" | "arrowdown" => Key::ArrowDown,
        other => {
            *FUNCTION_KEYS
                .iter()
                .enumerate()
                .find(|(idx, _)| other == format!("f{}", idx + 1))?
                .1
        }
    })
}

/// Human spelling of a [`Key`], inverse of [`parse_key`].
fn key_label(key: Key) -> &'static str {
    match key {
        Key::Plus => "+",
        Key::Equals => "=",
        Key::Minus => "-",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Slash => "/",
        Key::Escape => "Esc",
        Key::Enter => "Enter",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Insert => "Insert",
        Key::Delete => "Del",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PgUp",
        Key::PageDown => "PgDn",
        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::ArrowUp => "Up",
        Key::ArrowDown => "Down",
        _ => LETTERS
            .iter()
            .position(|candidate| candidate == &key)
            .map(|idx| LETTER_LABELS[idx])
            .or_else(|| {
                FUNCTION_KEYS
                    .iter()
                    .position(|candidate| candidate == &key)
                    .map(|idx| FUNCTION_LABELS[idx])
            })
            .or_else(|| {
                DIGITS
                    .iter()
                    .position(|d| d == &key)
                    .map(|idx| DIGIT_LABELS[idx])
            })
            .unwrap_or("?"),
    }
}

const LETTER_LABELS: [&str; 26] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z",
];

const FUNCTION_LABELS: [&str; 12] = [
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
];

const DIGIT_LABELS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

/// The effective keybind set: ordered `(binding, action)` claims, deduped
/// last-wins, plus the config path for the shortcuts-window hint row.
pub(crate) struct Keybinds {
    pub(crate) path: Option<PathBuf>,
    entries: Vec<(Binding, Action)>,
}

impl Keybinds {
    fn defaults(path: Option<PathBuf>) -> Keybinds {
        let entries = Action::ALL
            .into_iter()
            .flat_map(|action| {
                action
                    .defaults()
                    .iter()
                    .filter_map(move |spec| Some((Binding::parse(spec)?, action)))
            })
            .collect();
        Keybinds { path, entries }
    }

    /// Resolve a pressed key+modifiers to an action. Modifiers must match a
    /// binding exactly (`Ctrl+Q` never fires plain `Q`); ties between equal-
    /// specificity claims go to the later one.
    pub(crate) fn action_for(&self, key: Key, pressed: egui::Modifiers) -> Option<Action> {
        self.entries
            .iter()
            .filter(|(binding, _)| binding.key == key && binding.matches(pressed))
            .max_by_key(|(binding, _)| {
                u8::from(binding.ctrl)
                    + u8::from(binding.shift)
                    + u8::from(binding.alt)
                    + u8::from(binding.meta)
            })
            .map(|(_, action)| *action)
    }

    /// Shortcuts-window rows in canonical order: label + rendered bindings.
    /// Actions that lost every binding to conflicts render as unbound.
    pub(crate) fn rows(&self) -> Vec<(&'static str, String)> {
        Action::ALL
            .into_iter()
            .map(|action| {
                let mut keys = self
                    .entries
                    .iter()
                    .filter(|(_, claim)| *claim == action)
                    .map(|(binding, _)| binding.label())
                    .collect::<Vec<_>>()
                    .join(" / ");
                if keys.is_empty() {
                    keys.push('—');
                }
                (action.label(), keys)
            })
            .collect()
    }

    /// Load from `<config_dir>/keybinds.json`, seeding the file with the
    /// default document when missing. Bad JSON yields defaults without
    /// rewriting the file; per-entry problems fall back entry-wise.
    pub(crate) fn load_or_init(config_dir: Option<&Path>) -> Keybinds {
        let Some(dir) = config_dir else {
            return Keybinds::defaults(None);
        };
        let path = dir.join("keybinds.json");
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if let Err(err) = seed_defaults_file(dir, &path) {
                    warn(format!("seeding keybinds file: {err}"));
                }
                return Keybinds::defaults(Some(path));
            }
            Err(err) => {
                warn(format!("reading keybinds file: {err}"));
                return Keybinds::defaults(Some(path));
            }
        };
        match serde_json::from_str::<serde_json::Value>(&contents) {
            Ok(document @ serde_json::Value::Object(_)) => Self::merged(Some(path), &document),
            Ok(_) => {
                warn("keybinds file is not a JSON object — using defaults".to_owned());
                Keybinds::defaults(Some(path))
            }
            Err(err) => {
                warn(format!("corrupt keybinds file, using defaults: {err}"));
                Keybinds::defaults(Some(path))
            }
        }
    }

    /// Defaults overlaid with valid file entries; every corrupt or unknown
    /// entry warns and silently keeps that action's default bindings.
    fn merged(path: Option<PathBuf>, document: &serde_json::Value) -> Keybinds {
        let mut claims: Vec<(Binding, Action, bool)> = Keybinds::defaults(None)
            .entries
            .into_iter()
            .map(|(binding, action)| (binding, action, false))
            .collect();
        let object = document.as_object().expect("checked by caller");
        // Alphabetical iteration keeps conflict resolution deterministic;
        // later names win when two file actions claim one binding.
        let mut names: Vec<&String> = object.keys().collect();
        names.sort_unstable();
        for name in names {
            let Some(action) = Action::from_name(name) else {
                warn(format!("unknown action {name:?} in keybinds file"));
                continue;
            };
            let Some(specs) = value_specs(object.get(name)) else {
                warn(format!(
                    "entry {name:?} must be a key string or list of strings — keeping defaults"
                ));
                continue;
            };
            let parsed: Option<Vec<_>> = specs.iter().map(|spec| Binding::parse(spec)).collect();
            let Some(parsed) = parsed else {
                warn(format!(
                    "unparsable key in entry {name:?} ({specs:?}) — keeping defaults"
                ));
                continue;
            };
            claims.retain(|(_, owner, _)| *owner != action);
            claims.extend(parsed.into_iter().map(|binding| (binding, action, true)));
        }
        resolve_claims(path, claims)
    }
}

/// Whether this exact binding was shipped as a default — rebinding one of
/// those keys to a new action is intended use, not an ambiguity.
fn default_owner_of(binding: &Binding, defaults: &[(Binding, Action)]) -> Option<Action> {
    defaults
        .iter()
        .find(|(candidate, _)| candidate == binding)
        .map(|(_, action)| *action)
}

/// Settle shared bindings last-wins; dropped claims warn unless the clash is
/// just a file entry legitimately overwriting its old default.
fn resolve_claims(path: Option<PathBuf>, claims: Vec<(Binding, Action, bool)>) -> Keybinds {
    let defaults: Vec<(Binding, Action)> = claims
        .iter()
        .filter(|(.., from_file)| !from_file)
        .map(|(binding, action, _)| (*binding, *action))
        .collect();
    let mut settled: Vec<(Binding, Action)> = Vec::with_capacity(claims.len());
    for (binding, action, _) in claims {
        match settled.iter_mut().find(|(placed, _)| *placed == binding) {
            Some(slot) => {
                let replaced = slot.1;
                slot.1 = action;
                if replaced != action && default_owner_of(&binding, &defaults) != Some(replaced) {
                    warn(format!(
                        "key {} bound to both {} and {} — using {}",
                        binding.label(),
                        replaced,
                        action,
                        action,
                    ));
                }
            }
            None => settled.push((binding, action)),
        }
    }
    Keybinds {
        path,
        entries: settled,
    }
}

/// String (or array of strings) as this schema accepts binding values.
fn value_specs(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    match value? {
        serde_json::Value::String(key) => Some(vec![key.clone()]),
        serde_json::Value::Array(keys) => keys
            .iter()
            .map(|key| key.as_str().map(str::to_owned))
            .collect(),
        _ => None,
    }
}

/// Seed `path` with the documented default document so users can discover
/// and hand-edit their keybinds.
fn seed_defaults_file(dir: &Path, path: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    candi_core::write_file_atomically(path, &defaults_document())
}

/// The canonical default document: alphabetical, single bindings as plain
/// strings, multi-bindings as arrays.
fn defaults_document() -> String {
    use serde_json::Value;
    let mut map = serde_json::Map::new();
    for action in Action::ALL {
        let specs = action.defaults();
        map.insert(
            action.name().to_owned(),
            if specs.len() == 1 {
                Value::String(specs[0].to_owned())
            } else {
                Value::Array(
                    specs
                        .iter()
                        .map(|spec| Value::String((*spec).to_owned()))
                        .collect(),
                )
            },
        );
    }
    serde_json::to_string_pretty(&Value::Object(map)).expect("default keybinds serialize")
}

fn warn(message: String) {
    eprintln!("candi: {message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "candi-keybinds-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn every_default_spec_parses() {
        for action in Action::ALL {
            for spec in action.defaults() {
                let binding = Binding::parse(spec).unwrap_or_else(|| panic!("{spec:?}"));
                assert_eq!(binding.label(), *spec, "labels invert parse");
            }
        }
    }

    #[test]
    fn modifier_specs_parse_case_insensitively() {
        let binding = Binding::parse("CTRL+shift+s").expect("mixed case parses");
        assert!(binding.ctrl && binding.shift && !binding.alt && !binding.meta);
        assert_eq!(binding.key, Key::S);
        assert_eq!(binding.label(), "Ctrl+Shift+S");
    }

    #[test]
    fn bare_plus_and_duplicate_modifiers_parse() {
        assert_eq!(Binding::parse("+").map(|b| b.key), Some(Key::Plus));
        let doubled = Binding::parse("Ctrl+Ctrl+S").expect("duplicate modifiers");
        assert!(doubled.ctrl);
        assert_eq!(doubled.key, Key::S);
    }

    #[test]
    fn garbage_specs_are_rejected() {
        assert_eq!(Binding::parse(""), None);
        assert_eq!(Binding::parse("Ctrl+"), None);
        assert_eq!(Binding::parse("Ctrl+Bogus"), None);
        assert_eq!(Binding::parse("NotAKey"), None);
    }

    #[test]
    fn matching_requires_exact_modifier_bits() {
        let binding = Binding::parse("Ctrl+S").expect("parses");
        let mut mods = egui::Modifiers::default();
        assert!(!binding.matches(mods));
        mods.ctrl = true;
        assert!(binding.matches(mods));
        mods.shift = true;
        assert!(
            !binding.matches(mods),
            "Ctrl+Shift+S is not Ctrl+S — exact bits"
        );

        let plain = Binding::parse("Q").expect("parses");
        assert!(plain.matches(egui::Modifiers::NONE));
        assert!(!plain.matches(egui::Modifiers::CTRL), "no bleed-through");
    }

    #[test]
    fn missing_file_is_seeded_with_defaults() {
        let dir = temp_dir("seed");
        let keybinds = Keybinds::load_or_init(Some(&dir));
        let contents = fs::read_to_string(dir.join("keybinds.json")).expect("seeded");
        let doc: serde_json::Value = serde_json::from_str(&contents).expect("valid json");
        assert_eq!(
            doc["next_page"],
            serde_json::json!(["Right", "PgDn"]),
            "multi-bindings serialize as arrays"
        );
        assert_eq!(doc["quit"], serde_json::json!("Q"));
        // The seeded file round-trips to the same bindings as the defaults.
        for (label, keys) in keybinds.rows() {
            assert!(!keys.is_empty(), "{label} lost its default bindings");
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn valid_entries_override_defaults() {
        let dir = temp_dir("override");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("keybinds.json"),
            r#"{"quit": "Ctrl+Shift+X", "open_file": ["O", "Ctrl+O"]}"#,
        )
        .unwrap();
        let keybinds = Keybinds::load_or_init(Some(&dir));
        assert_eq!(
            keybinds.action_for(Key::X, egui::Modifiers::CTRL.plus(egui::Modifiers::SHIFT)),
            Some(Action::Quit)
        );
        assert_eq!(
            keybinds.action_for(Key::O, egui::Modifiers::NONE),
            Some(Action::OpenFile)
        );
        // Unlisted actions keep their defaults.
        assert_eq!(
            keybinds.action_for(Key::Minus, egui::Modifiers::default()),
            Some(Action::ZoomOut)
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupt_json_yields_defaults_without_rewriting() {
        let dir = temp_dir("corrupt");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("keybinds.json"), "{not json").unwrap();
        let keybinds = Keybinds::load_or_init(Some(&dir));
        assert_eq!(
            keybinds.action_for(Key::Q, egui::Modifiers::default()),
            Some(Action::Quit)
        );
        assert_eq!(
            fs::read_to_string(dir.join("keybinds.json")).unwrap(),
            "{not json",
            "a hand-editable corrupt file is left alone"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unknown_action_and_garbage_entries_fall_back() {
        let dir = temp_dir("fallback");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("keybinds.json"),
            r#"{"explode": "X", "quit": "Not~A~Key", "zoom_out": 7}"#,
        )
        .unwrap();
        let keybinds = Keybinds::load_or_init(Some(&dir));
        assert_eq!(
            keybinds.action_for(Key::Q, egui::Modifiers::default()),
            Some(Action::Quit)
        );
        assert_eq!(
            keybinds.action_for(Key::Minus, egui::Modifiers::default()),
            Some(Action::ZoomOut)
        );
        assert!(
            keybinds
                .entries
                .iter()
                .all(|(binding, _)| binding.key != Key::X),
            "unknown actions claim nothing"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn conflicting_claims_resolve_last_wins() {
        let dir = temp_dir("conflict");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("keybinds.json"),
            r#"{"bookmark": "Z", "quit": "Z"}"#,
        )
        .unwrap();
        let keybinds = Keybinds::load_or_init(Some(&dir));
        assert_eq!(
            keybinds.action_for(Key::Z, egui::Modifiers::default()),
            Some(Action::Quit),
            "alphabetically later entries win"
        );
        let (_, bookmark_keys) = keybinds
            .rows()
            .into_iter()
            .find(|(label, _)| *label == "Bookmark page")
            .expect("row exists");
        assert_eq!(bookmark_keys, "—", "the losing action shows unbound");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn non_object_documents_yield_defaults() {
        let dir = temp_dir("arraydoc");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("keybinds.json"), "[1, 2]").unwrap();
        let keybinds = Keybinds::load_or_init(Some(&dir));
        assert_eq!(
            keybinds.action_for(Key::T, egui::Modifiers::default()),
            Some(Action::CycleTheme)
        );
        fs::remove_dir_all(&dir).ok();
    }
}
