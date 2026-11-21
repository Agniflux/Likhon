//! Fixed keyboard layouts (Probhat, Jatiya, Munir...): each key position types what the layout file says.
//!
//! Layouts are JSON files only (shared/schemas/layout.schema.json, owner decision 2026-10-22). This module parses
//! them strictly and types by key position, not by the character the US keyboard would give, so the same key
//! works whatever Windows keyboard is installed.
//!
//! Automatic vowel forming (as Avro does it): a vowel sign (kar) typed where it cannot attach, at the start of a
//! word or after anything that is not a consonant, becomes the full vowel: া at a word start gives আ. Layouts such
//! as Jatiya and Munir have no keys of their own for most full vowels and rely on this. A vowel sign right after a
//! hasant replaces it with the full vowel (ক ্ া gives কআ), and hasant then । gives ॥: UniJoy's dead hasant.

use crate::Action;
use std::collections::HashMap;

/// The largest layout file read (the schema's keys and texts are small; Avro's converted files are about 10 KB).
pub const MAX_LAYOUT_BYTES: usize = 64 * 1024;

/// One key press for a fixed layout: the key's position, named by the character it types on a US keyboard
/// unshifted (`a`..`z`, `0`..`9`, `` ` - = [ ] \ ; ' , . / ``), and the level keys held with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyPress {
    pub key: char,
    pub shift: bool,
    /// AltGr (right Alt) is held.
    pub altgr: bool,
}

#[derive(Debug, Default, Clone)]
struct Levels {
    normal: Option<String>,
    shift: Option<String>,
    altgr: Option<String>,
    shift_altgr: Option<String>,
}

/// A parsed layout file.
#[derive(Debug, Clone)]
pub struct Layout {
    pub id: String,
    pub name_en: String,
    pub name_bn: Option<String>,
    keys: HashMap<char, Levels>,
}

const KEY_NAMES: &str = "abcdefghijklmnopqrstuvwxyz0123456789`-=[]\\;',./";
/// LicenseRef-owner-approved: a layout the owner chose to ship although its file names no licence (UniJoy, owner
/// decision 2026-10-23; SOURCE.md beside it says where it came from).
const LICENSES: [&str; 7] = ["MPL-2.0", "MIT", "CC0-1.0", "CC-BY-4.0", "OFL-1.1", "Apache-2.0", "LicenseRef-owner-approved"];

impl Layout {
    /// Parses a layout file. Anything outside the schema is an error: unknown fields, unknown keys, long texts,
    /// a licence not on the allowed list. Layouts that ask for vowel reordering are refused until the engine has it.
    pub fn from_json(text: &str) -> Result<Layout, String> {
        if text.len() > MAX_LAYOUT_BYTES {
            return Err("the layout file is too large".into());
        }
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("not valid JSON: {e}"))?;
        let o = v.as_object().ok_or("the layout is not a JSON object")?;
        const FIELDS: [&str; 13] = ["format", "version", "id", "name", "kind", "author", "description", "layoutVersion", "license", "source", "preBaseVowels", "keys", "numpad"];
        if let Some(k) = o.keys().find(|k| !FIELDS.contains(&k.as_str())) {
            return Err(format!("unknown field \"{k}\""));
        }
        if o.get("format").and_then(|f| f.as_str()) != Some("likhon-layout") || o.get("version").and_then(|v| v.as_u64()) != Some(1) {
            return Err("not a Likhon layout, version 1".into());
        }
        let id = o.get("id").and_then(|v| v.as_str()).ok_or("id missing")?;
        if id.is_empty() || id.len() > 40 || !id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
            return Err("id must be lower-case letters, digits and hyphens".into());
        }
        if o.get("kind").and_then(|v| v.as_str()) != Some("fixed") {
            return Err("kind must be \"fixed\"".into());
        }
        if !o.get("license").and_then(|v| v.as_str()).is_some_and(|l| LICENSES.contains(&l)) {
            return Err("licence missing or not on the allowed list".into());
        }
        if o.get("preBaseVowels").and_then(|v| v.as_bool()) == Some(true) {
            return Err("layouts that type vowel signs before the consonant are not supported yet".into());
        }
        let name = o.get("name").and_then(|v| v.as_object()).ok_or("name missing")?;
        let name_en = name.get("en").and_then(|v| v.as_str()).filter(|s| !s.is_empty() && s.chars().count() <= 60).ok_or("name.en missing")?;
        let name_bn = name.get("bn").and_then(|v| v.as_str()).filter(|s| !s.is_empty() && s.chars().count() <= 60);

        let mut keys = HashMap::new();
        for (k, levels) in o.get("keys").and_then(|v| v.as_object()).ok_or("keys missing")? {
            let mut chars = k.chars();
            let key = match (chars.next(), chars.next()) {
                (Some(c), None) if KEY_NAMES.contains(c) => c,
                _ => return Err(format!("unknown key \"{k}\"")),
            };
            let lv = levels.as_object().ok_or_else(|| format!("key \"{k}\" is not an object"))?;
            let mut out = Levels::default();
            for (level, value) in lv {
                let s = value.as_str().filter(|s| !s.is_empty() && s.chars().count() <= 8 && !s.chars().any(|c| c.is_control())).ok_or_else(|| format!("key \"{k}\" {level}: 1 to 8 characters"))?;
                let slot = match level.as_str() {
                    "normal" => &mut out.normal,
                    "shift" => &mut out.shift,
                    "altgr" => &mut out.altgr,
                    "shiftAltgr" => &mut out.shift_altgr,
                    _ => return Err(format!("key \"{k}\": unknown level \"{level}\"")),
                };
                *slot = Some(s.to_string());
            }
            keys.insert(key, out);
        }
        Ok(Layout { id: id.to_string(), name_en: name_en.to_string(), name_bn: name_bn.map(String::from), keys })
    }

    /// What the layout gives for this key press, before vowel forming.
    pub fn lookup(&self, k: KeyPress) -> Option<&str> {
        let l = self.keys.get(&k.key)?;
        match (k.shift, k.altgr) {
            (false, false) => l.normal.as_deref(),
            (true, false) => l.shift.as_deref(),
            (false, true) => l.altgr.as_deref(),
            (true, true) => l.shift_altgr.as_deref(),
        }
    }
}

fn is_consonant(c: char) -> bool {
    matches!(c, '\u{0995}'..='\u{09B9}' | '\u{09DC}' | '\u{09DD}' | '\u{09DF}' | '\u{09BC}')
}

/// The full vowel for a vowel sign, for automatic vowel forming.
fn full_vowel(kar: char) -> Option<char> {
    Some(match kar {
        'া' => 'আ',
        'ি' => 'ই',
        'ী' => 'ঈ',
        'ু' => 'উ',
        'ূ' => 'ঊ',
        'ৃ' => 'ঋ',
        'ে' => 'এ',
        'ৈ' => 'ঐ',
        'ো' => 'ও',
        'ৌ' => 'ঔ',
        _ => return None,
    })
}

/// Typing with a fixed layout, one key press at a time.
pub struct Fixed {
    layout: Layout,
    /// The last character typed in the current word; None at a word start or after a reset.
    prev: Option<char>,
}

impl Fixed {
    pub fn new(layout: Layout) -> Self {
        Self { layout, prev: None }
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn key(&mut self, k: KeyPress) -> Action {
        let Some(out) = self.layout.lookup(k) else {
            self.reset();
            return Action::PassThrough;
        };
        let mut chars = out.chars();
        let single = match (out.chars().next(), out.chars().nth(1)) {
            (Some(c), None) => Some(c),
            _ => None,
        };
        if self.prev == Some('\u{09CD}') {
            if let Some(full) = single.and_then(|c| if c == '।' { Some('॥') } else { full_vowel(c) }) {
                self.prev = Some(full);
                return Action::Replace { backspaces: 1, output: full.to_string() };
            }
        }
        let output = match (chars.next(), chars.next()) {
            (Some(c), None) if !self.prev.is_some_and(is_consonant) => full_vowel(c).map(String::from).unwrap_or_else(|| out.to_string()),
            _ => out.to_string(),
        };
        self.prev = output.chars().last();
        Action::Replace { backspaces: 0, output }
    }

    /// The caret may have moved or a word ended (space, Enter, a click): the next vowel sign starts fresh.
    pub fn reset(&mut self) {
        self.prev = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROBHAT: &str = include_str!("../../../assets/layouts/avro/probhat.json");
    const JATIYA: &str = include_str!("../../../assets/layouts/avro/jatiya.json");
    const MUNIR: &str = include_str!("../../../assets/layouts/avro/munir.json");
    const AVRO_EASY: &str = include_str!("../../../assets/layouts/avro/avro-easy.json");
    /// UniJoy's file is not in the public repository (it states no licence), so its tests run only where it exists.
    fn unijoy() -> Option<String> {
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/layouts/unijoy/unijoy.json")).ok()
    }

    fn press(key: char) -> KeyPress {
        KeyPress { key, shift: false, altgr: false }
    }

    /// Types key presses ("K" means Shift+k) and returns the text.
    fn type_into(layout: &str, keys: &str) -> String {
        let mut f = Fixed::new(Layout::from_json(layout).unwrap());
        let mut screen = String::new();
        for c in keys.chars() {
            let k = if c.is_ascii_uppercase() { KeyPress { key: c.to_ascii_lowercase(), shift: true, altgr: false } } else { press(c) };
            match f.key(k) {
                Action::PassThrough => {
                    screen.push(c);
                    f.reset();
                }
                Action::Replace { backspaces, output } => {
                    for _ in 0..backspaces {
                        screen.pop();
                    }
                    screen.push_str(&output);
                }
            }
        }
        screen
    }

    #[test]
    fn every_bundled_layout_parses() {
        let unijoy = unijoy();
        let mut layouts = vec![(PROBHAT, "probhat"), (JATIYA, "jatiya"), (MUNIR, "munir"), (AVRO_EASY, "avro-easy")];
        if let Some(u) = unijoy.as_deref() {
            layouts.push((u, "unijoy"));
        }
        for (text, id) in layouts {
            let l = Layout::from_json(text).unwrap();
            assert_eq!(l.id, id);
            assert_eq!(l.keys.len(), 47, "{id}");
        }
    }

    #[test]
    fn unijoy_types_with_its_dead_hasant_and_reph() {
        let Some(unijoy) = unijoy() else { return };
        let UNIJOY = unijoy.as_str();
        // UniJoy: j ক, f া, m ম, d ি, g ্ (hasant + vowel sign gives the full vowel), Shift+A র্, Z ্র, Shift+Z ্য.
        assert_eq!(type_into(UNIJOY, "jfm"), "কাম");
        assert_eq!(type_into(UNIJOY, "gfmd"), "আমি");
        assert_eq!(type_into(UNIJOY, "Amf"), "র্মা");
        assert_eq!(type_into(UNIJOY, "rzf"), "প্রা");
        assert_eq!(type_into(UNIJOY, "bZ"), "ন্য");
        assert_eq!(type_into(UNIJOY, "gG"), "॥");
        assert_eq!(type_into(UNIJOY, "jgj"), "ক্ক", "hasant before a consonant stays");
    }

    #[test]
    fn probhat_types_by_key_position() {
        // Probhat: k ক, a া, m ম, l ল.
        assert_eq!(type_into(PROBHAT, "kml"), "কমল");
        assert_eq!(type_into(PROBHAT, "kam"), "কাম");
    }

    #[test]
    fn jatiya_types_by_key_position() {
        // Jatiya: j ক, h া, f ব.
        assert_eq!(type_into(JATIYA, "jhf"), "কাব");
    }

    #[test]
    fn a_vowel_sign_at_a_word_start_becomes_the_full_vowel() {
        // Jatiya h is া: alone it gives আ; after ক it stays া.
        assert_eq!(type_into(JATIYA, "h"), "আ");
        assert_eq!(type_into(JATIYA, "jh"), "কা");
        assert_eq!(type_into(JATIYA, "j h"), "ক আ");
    }

    #[test]
    fn levels_follow_shift_and_altgr() {
        let l = Layout::from_json(PROBHAT).unwrap();
        assert_eq!(l.lookup(press('1')), Some("১"));
        assert_eq!(l.lookup(KeyPress { key: '1', shift: true, altgr: false }), Some("!"));
        assert_eq!(l.lookup(KeyPress { key: '1', shift: false, altgr: true }), Some("৴"));
    }

    #[test]
    fn strict_parsing_refuses_what_the_schema_does_not_allow() {
        let ok = r#"{"format":"likhon-layout","version":1,"id":"t","name":{"en":"T"},"kind":"fixed","license":"MIT","keys":{"k":{"normal":"ক"}}}"#;
        assert!(Layout::from_json(ok).is_ok());
        for bad in [
            ok.replace("\"kind\"", "\"script\":\"x\",\"kind\""),
            ok.replace("\"MIT\"", "\"Proprietary\""),
            ok.replace("\"k\":", "\"F1\":"),
            ok.replace("\"normal\"", "\"hyper\""),
            ok.replace("\"ক\"", &format!("\"{}\"", "ক".repeat(9))),
            ok.replace("\"fixed\"", "\"phonetic\""),
        ] {
            assert!(Layout::from_json(&bad).is_err(), "{bad}");
        }
        assert!(Layout::from_json(&" ".repeat(MAX_LAYOUT_BYTES + 1)).is_err());
    }
}
