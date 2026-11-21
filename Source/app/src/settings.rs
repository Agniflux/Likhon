//! settings.json (Desktop/CLAUDE.md section 2, data location rule): next to the exe when that folder is
//! writable, otherwise in the user's profile (%LOCALAPPDATA%\Likhon).
//!
//! One flat JSON object. Only keys that differ from `defaults()` need to be stored; a missing key means
//! the default. The Settings window and the agent both write the file, each only the keys it owns, by
//! reading it fresh and changing one key at a time (`update`), so neither overwrites the other's changes.
//! The schema goes to shared/schemas with M6.

use crate::text::Lang;
use likhon_engine::Options;
use serde_json::{json, Map, Value};
use std::path::PathBuf;

/// Every setting and its default. The Settings window gets this list too (boot.json).
pub fn defaults() -> Map<String, Value> {
    let v = json!({
        // General (S01)
        "language": default_language(),
        "color": "system",
        "topbar.mode": "always",
        // General: fade the top bar when idle ("never", "40" percent, "hide")
        "topbar.fade": "never",
        "start_with_windows": false,
        // Typing (S02)
        "typing.suggestions": true,
        "typing.suggestion_count": 5,
        "typing.next_word": true,
        "typing.next_count": 3,
        "typing.autocorrect": true,
        "typing.emoji": false,
        "typing.bangla_digits": false,
        "typing.dari": true,
        "typing.visarga": true,
        "typing.number_pick": false,
        "typing.learn": true,
        // Shortcuts (v2 §7): expand on Space; off means Tab only
        "shortcuts.space": true,
        // Typing: what Space types when the suggestion list is open ("dict": a dictionary spelling, "plain": as typed)
        "typing.preselect": "dict",
        // Suggestions (v2 §7): follow the text cursor, or stay anchored under the top bar; caret badge on switching
        "suggestions.position": "follow",
        "suggestions.caret_badge": true,
        // Layouts and output (S03, S04)
        "layout": "phonetic",
        "output": "unicode",
        // Legacy output switched on (Settings › Legacy output): then Shift+F12 and the top bar pill flip the output.
        "legacy.enabled": false,
        // Which legacy encoding: "bijoy" or "proshika" (app/src/legacy.rs).
        "legacy.encoding": "bijoy",
        // Hotkeys (S05)
        "hotkeys.toggle": { "vk": 0x7B, "ctrl": false, "alt": false, "shift": false, "label": "F12" },
        // Hotkeys: next layout (Phonetic, then the fixed layouts); none until the person sets one
        "hotkeys.next_layout": null,
        // App rules (S06): F12 goes to these apps (the design's defaults)
        "apprules.passthrough": [
            { "name": "Google Chrome", "exe": "chrome.exe" },
            { "name": "Mozilla Firefox", "exe": "firefox.exe" },
            { "name": "Visual Studio Code", "exe": "Code.exe" }
        ],
        // Appearance (S07): null follows Colors (Default light or Default dark)
        "appearance.theme": null,
        "appearance.font": null,
        "appearance.font_size": 17,
        "appearance.row_height": 30,
        // Kept by the agent
        "start_bangla": false,
        "tray_hint_shown": false,
        "tray_notice_shown": false,
        "topbar.x": null,
        "topbar.y": null,
        "topbar.hidden": false
    });
    v.as_object().cloned().unwrap_or_default()
}

/// Assumed default (root CLAUDE.md, open decisions): Bangla if Windows is set to Bangla, otherwise English.
fn default_language() -> &'static str {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    // Primary language ID 0x45 = Bengali.
    if unsafe { GetUserDefaultUILanguage() } & 0x3FF == 0x45 { "bn" } else { "en" }
}

/// The Likhon website the app talks to and opens pages of: likhon.test for now (owner decision 2026-10-23; the site
/// lives there until likhon.app is ready). settings.json may set "site" to another https address; nothing else is
/// accepted, so a changed file can never point the app at plain http or a non-web scheme.
/// A build can name another site with the LIKHON_SITE environment variable (scripts/make-portable.ps1 -Site).
pub const DEFAULT_SITE: &str = match option_env!("LIKHON_SITE") {
    Some(s) => s,
    None => "https://likhon.test",
};

pub fn site() -> String {
    let s = stored().get("site").and_then(Value::as_str).map(str::to_string).unwrap_or_default();
    let ok = s.starts_with("https://") && s.len() < 100 && s[8..].bytes().all(|b| b.is_ascii_alphanumeric() || b".-:".contains(&b));
    if ok { s } else { DEFAULT_SITE.to_string() }
}

/// The folder for settings.json and other data: beside the exe if writable, else the profile.
/// The dictionary: dict/words.sqlite beside likhon.exe (owner decision 2026-10-22: a separate dict folder).
pub fn dictionary_path() -> Option<PathBuf> {
    let p = std::env::current_exe().ok()?.parent()?.join("dict").join("words.sqlite");
    p.is_file().then_some(p)
}

pub fn dir() -> (PathBuf, bool) {
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        let probe = dir.join(".likhon-write-test");
        if std::fs::write(&probe, b"").is_ok() {
            let _ = std::fs::remove_file(&probe);
            return (dir, true);
        }
    }
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let dir = base.join("Likhon");
    let _ = std::fs::create_dir_all(&dir);
    (dir, false)
}

pub fn path() -> PathBuf {
    dir().0.join("settings.json")
}

/// The stored keys (not the defaults). A broken or missing file reads as empty.
pub fn stored() -> Map<String, Value> {
    // A byte-order mark (some editors and PowerShell add one) would make the whole file unreadable: skip it.
    std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str::<Value>(s.trim_start_matches('\u{feff}')).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

/// Read the file, change it, write it back (through a temporary file, so a crash never leaves half a file).
pub fn update(change: impl FnOnce(&mut Map<String, Value>)) {
    let mut m = stored();
    change(&mut m);
    let defaults = defaults();
    m.retain(|k, v| defaults.get(k) != Some(v)); // keep the file to what differs from the defaults
    let body = serde_json::to_string_pretty(&Value::Object(m)).unwrap_or_else(|_| "{}".into()) + "\n";
    let p = path();
    let tmp = p.with_extension("json.tmp");
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, &p);
    }
}

pub fn set(key: &str, value: Value) {
    update(|m| {
        m.insert(key.to_string(), value);
    });
}

/// The toggle key (Hotkeys page). Default F12 with no modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub vk: u16,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// What the agent needs, read from the file plus defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub language: Lang,
    pub color: String,
    pub topbar_mode: String,
    pub engine: Options,
    pub layout: String,
    pub output: String,
    /// Legacy output is switched on: Shift+F12 flips between Bijoy and Unicode.
    pub legacy_enabled: bool,
    /// The legacy encoding ("bijoy" or "proshika"); `output` is "bijoy" whenever legacy output is on, whichever it is.
    pub legacy_encoding: String,
    pub toggle: Hotkey,
    pub toggle_label: String,
    /// Hotkeys: next layout, if set.
    pub next_layout: Option<Hotkey>,
    /// General: "never", "40" or "hide".
    pub topbar_fade: String,
    /// Lower-case exe names that get F12 for themselves (App rules).
    pub passthrough: Vec<String>,
    pub theme: Option<String>,
    /// Typing: autocorrect common misspellings (and the person's own autocorrect list).
    pub autocorrect: bool,
    /// Typing: learn words typed in fields confirmed not to be password fields.
    pub learn: bool,
    /// Typing: emoji by name (:hashi:).
    pub emoji: bool,
    /// Typing: suggest the next word after a word ends.
    pub next_word: bool,
    /// Shortcuts: expand on Space as well as Tab.
    pub shortcuts_space: bool,
    /// Suggestions: show the window, how many rows, Space picks a dictionary spelling, anchored, caret badge.
    pub suggestions: bool,
    pub suggestion_count: usize,
    /// Suggestions: how many next words.
    pub next_count: usize,
    pub preselect_dict: bool,
    /// Suggestions: the number keys 1 to 9 take that row while a word is being typed.
    pub number_pick: bool,
    pub suggest_anchored: bool,
    pub caret_badge: bool,
    /// Appearance: the suggestion window's font (an installed Bangla font), or Windows' usual one.
    pub font: Option<String>,
    /// Appearance: suggestion text size in DIPs.
    pub font_size: f32,
    pub start_bangla: bool,
    pub tray_hint_shown: bool,
    pub tray_notice_shown: bool,
    pub topbar_pos: Option<(i32, i32)>,
    pub topbar_hidden: bool,
}

impl Default for Settings {
    fn default() -> Self {
        from_map(&Map::new())
    }
}

pub fn load() -> Settings {
    from_map(&stored())
}

fn from_map(stored: &Map<String, Value>) -> Settings {
    let d = defaults();
    let get = |k: &str| stored.get(k).or_else(|| d.get(k)).cloned().unwrap_or(Value::Null);
    let s = |k: &str| get(k).as_str().unwrap_or_default().to_string();
    let b = |k: &str| get(k).as_bool().unwrap_or(false);
    let hk = get("hotkeys.toggle");
    let toggle = Hotkey {
        vk: hk["vk"].as_u64().filter(|v| (1..255).contains(v)).unwrap_or(0x7B) as u16,
        ctrl: hk["ctrl"].as_bool().unwrap_or(false),
        alt: hk["alt"].as_bool().unwrap_or(false),
        shift: hk["shift"].as_bool().unwrap_or(false),
    };
    let pos = match (get("topbar.x").as_i64(), get("topbar.y").as_i64()) {
        (Some(x), Some(y)) => Some((x as i32, y as i32)),
        _ => None,
    };
    Settings {
        language: if s("language") == "bn" { Lang::Bn } else { Lang::En },
        color: s("color"),
        topbar_mode: s("topbar.mode"),
        engine: Options { bangla_digits: b("typing.bangla_digits"), dari: b("typing.dari"), visarga: b("typing.visarga") },
        layout: s("layout"),
        output: s("output"),
        // Files from before the switch existed: Bijoy output means legacy is on.
        legacy_enabled: b("legacy.enabled") || s("output") == "bijoy",
        legacy_encoding: if s("legacy.encoding") == "proshika" { "proshika".into() } else { "bijoy".into() },
        toggle,
        toggle_label: hk["label"].as_str().unwrap_or("F12").to_string(),
        next_layout: get("hotkeys.next_layout").as_object().and_then(|o| {
            let vk = o.get("vk")?.as_u64().filter(|v| (1..255).contains(v))? as u16;
            let flag = |k: &str| o.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
            Some(Hotkey { vk, ctrl: flag("ctrl"), alt: flag("alt"), shift: flag("shift") })
        }),
        topbar_fade: s("topbar.fade"),
        passthrough: get("apprules.passthrough").as_array().map(|a| a.iter().filter_map(|x| x["exe"].as_str()).map(|e| e.to_ascii_lowercase()).collect()).unwrap_or_default(),
        theme: get("appearance.theme").as_str().map(String::from),
        autocorrect: b("typing.autocorrect"),
        learn: b("typing.learn"),
        emoji: b("typing.emoji"),
        next_word: b("typing.next_word"),
        shortcuts_space: b("shortcuts.space"),
        suggestions: b("typing.suggestions"),
        suggestion_count: get("typing.suggestion_count").as_u64().unwrap_or(5) as usize,
        next_count: get("typing.next_count").as_u64().unwrap_or(3) as usize,
        preselect_dict: s("typing.preselect") != "plain",
        number_pick: b("typing.number_pick"),
        suggest_anchored: s("suggestions.position") == "anchored",
        caret_badge: b("suggestions.caret_badge"),
        font: get("appearance.font").as_str().filter(|f| !f.is_empty() && f.len() <= 64).map(String::from),
        font_size: get("appearance.font_size").as_f64().unwrap_or(17.0) as f32,
        start_bangla: b("start_bangla"),
        tray_hint_shown: b("tray_hint_shown"),
        tray_notice_shown: b("tray_notice_shown"),
        topbar_pos: pos,
        topbar_hidden: b("topbar.hidden"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_keys_use_the_defaults() {
        let s = from_map(&Map::new());
        assert_eq!(s.toggle, Hotkey { vk: 0x7B, ctrl: false, alt: false, shift: false });
        assert_eq!(s.topbar_mode, "always");
        assert!(s.engine.dari && !s.engine.bangla_digits);
        assert_eq!(s.passthrough, vec!["chrome.exe", "firefox.exe", "code.exe"]);
    }

    #[test]
    fn stored_keys_win() {
        let m = json!({ "language": "bn", "typing.bangla_digits": true, "hotkeys.toggle": { "vk": 75, "ctrl": true, "alt": true, "label": "Ctrl + Alt + K" }, "topbar.x": 10, "topbar.y": 20 });
        let s = from_map(m.as_object().unwrap());
        assert_eq!(s.language, Lang::Bn);
        assert!(s.engine.bangla_digits);
        assert_eq!(s.toggle, Hotkey { vk: 75, ctrl: true, alt: true, shift: false });
        assert_eq!(s.toggle_label, "Ctrl + Alt + K");
        assert_eq!(s.topbar_pos, Some((10, 20)));
    }

    #[test]
    fn broken_values_fall_back() {
        let m = json!({ "hotkeys.toggle": "nonsense", "apprules.passthrough": 5 });
        let s = from_map(m.as_object().unwrap());
        assert_eq!(s.toggle.vk, 0x7B);
        assert!(s.passthrough.is_empty());
    }
}
