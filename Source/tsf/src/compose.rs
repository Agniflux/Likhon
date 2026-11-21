//! What the text service types: the word being composed, kept as the keys pressed and rendered again from the start
//! after every key (so Backspace inside a word is exact). The same engine and layouts as the agent's hook path.

use likhon_engine::fixed::{Fixed, KeyPress, Layout};
use likhon_engine::mit::khipro::{InputMethod, Khipro};
use likhon_engine::{Action, Options, Phonetic};
use serde_json::Value;
use std::path::PathBuf;

/// The layouts built into likhon.exe (app/src/layouts.rs), the same files. Others come from the library into the data
/// folder's layouts/ (JSON layouts, and input methods such as Khipro as <id>.mim).
pub const BUILT_IN: &[(&str, &str)] = &[
    ("probhat", include_str!("../../assets/layouts/avro/probhat.json")),
    ("jatiya", include_str!("../../assets/layouts/avro/jatiya.json")),
];

#[derive(Clone)]
pub enum Scheme {
    Phonetic(Options),
    Fixed(Layout),
    Khipro(std::sync::Arc<InputMethod>),
}

/// One key of the word: the character it types on a US keyboard (phonetic, Khipro) and its position (fixed layouts).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub ch: char,
    pub press: KeyPress,
}

/// The toggle key (Settings › General), F12 unless changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hotkey {
    pub vk: u16,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

pub struct Composer {
    pub scheme: Scheme,
    keys: Vec<Key>,
}

/// Plays `keys` into a fresh engine. Returns the text and whether the last key was not the engine's to take.
pub fn render(scheme: &Scheme, keys: &[Key]) -> (String, bool) {
    let mut screen: Vec<u16> = Vec::new();
    let mut passed = false;
    let apply = |a: Action, ch: char, screen: &mut Vec<u16>| match a {
        Action::PassThrough => {
            let mut b = [0u16; 2];
            screen.extend_from_slice(ch.encode_utf16(&mut b));
            true
        }
        Action::Replace { backspaces, output } => {
            screen.truncate(screen.len().saturating_sub(backspaces));
            screen.extend(output.encode_utf16());
            false
        }
    };
    match scheme {
        Scheme::Phonetic(o) => {
            let mut e = Phonetic::with_options(*o);
            for k in keys {
                passed = apply(e.key(k.ch), k.ch, &mut screen);
            }
        }
        Scheme::Fixed(l) => {
            let mut f = Fixed::new(l.clone());
            for k in keys {
                passed = apply(f.key(k.press), k.ch, &mut screen);
            }
        }
        Scheme::Khipro(im) => {
            let mut e = Khipro::new(im.clone());
            for k in keys {
                passed = apply(e.key(k.ch), k.ch, &mut screen);
            }
        }
    }
    (String::from_utf16_lossy(&screen), passed)
}

impl Composer {
    pub fn new(scheme: Scheme) -> Self {
        Self { scheme, keys: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The word with this key added, or None when the key is not part of a word (it goes to the app as it is).
    pub fn with(&self, k: Key) -> Option<String> {
        if self.keys.len() >= 64 {
            return None;
        }
        let mut keys = self.keys.clone();
        keys.push(k);
        let (text, passed) = render(&self.scheme, &keys);
        (!passed).then_some(text)
    }

    pub fn push(&mut self, k: Key) -> String {
        self.keys.push(k);
        self.text()
    }

    pub fn pop(&mut self) -> String {
        self.keys.pop();
        self.text()
    }

    pub fn clear(&mut self) {
        self.keys.clear();
    }

    pub fn text(&self) -> String {
        render(&self.scheme, &self.keys).0
    }

    /// The keys as typed on a US keyboard ("ami"), for the person's own word choices.
    pub fn typed(&self) -> String {
        self.keys.iter().map(|k| k.ch).collect()
    }
}

/// The character a key gives on a US keyboard, as the agent reads keys (app/src/agent.rs).
pub fn us_char(vk: u16, shift: bool, caps: bool) -> Option<char> {
    let c = match vk {
        0x41..=0x5A => {
            let lower = (b'a' + (vk - 0x41) as u8) as char;
            return Some(if shift ^ caps { lower.to_ascii_uppercase() } else { lower });
        }
        0x30..=0x39 => {
            let i = (vk - 0x30) as usize;
            return Some(if shift { b")!@#$%^&*("[i] as char } else { (b'0' + i as u8) as char });
        }
        0xBA => (';', ':'),
        0xBB => ('=', '+'),
        0xBC => (',', '<'),
        0xBD => ('-', '_'),
        0xBE => ('.', '>'),
        0xBF => ('/', '?'),
        0xC0 => ('`', '~'),
        0xDB => ('[', '{'),
        0xDC => ('\\', '|'),
        0xDD => (']', '}'),
        0xDE => ('\'', '"'),
        _ => return None,
    };
    Some(if shift { c.1 } else { c.0 })
}

/// A key for the composer, or None for keys that are not typing keys (Space, Enter, arrows, function keys).
pub fn key(vk: u16, shift: bool, caps: bool, altgr: bool) -> Option<Key> {
    let ch = us_char(vk, shift, caps)?;
    let pos = us_char(vk, false, false)?;
    Some(Key { ch, press: KeyPress { key: pos, shift, altgr } })
}

// ---- Settings (settings.json, written by likhon.exe) ------------------------------------------------------------

/// Where likhon.exe keeps its data: the folder named in datadir.txt beside the DLL (scripts/register-tsf.ps1 writes
/// it: the Portable folder), the DLL's own folder (installed beside likhon.exe), or the profile folder.
pub fn data_dirs(dll_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    if let Some(d) = &dll_dir {
        let named = std::fs::read_to_string(d.join("datadir.txt")).ok().and_then(|t| t.trim_start_matches('\u{feff}').lines().next().map(|l| l.trim().to_string()));
        if let Some(line) = named.filter(|l| !l.is_empty()) {
            v.push(PathBuf::from(line));
        }
    }
    v.extend(dll_dir);
    if let Some(l) = std::env::var_os("LOCALAPPDATA") {
        v.push(PathBuf::from(l).join("Likhon"));
    }
    v
}

pub struct Loaded {
    pub scheme: Scheme,
    pub toggle: Hotkey,
}

/// Reads settings.json (missing or unreadable, as in a Store app's sandbox: phonetic with the defaults).
pub fn load(dirs: &[PathBuf]) -> Loaded {
    let (dir, s) = dirs
        .iter()
        .find_map(|d| std::fs::read_to_string(d.join("settings.json")).ok().and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok()).map(|v| (Some(d.clone()), v)))
        .unwrap_or((None, Value::Null));
    let b = |k: &str, d: bool| s.get(k).and_then(Value::as_bool).unwrap_or(d);
    let options = Options { bangla_digits: b("typing.bangla_digits", false), dari: b("typing.dari", true), visarga: b("typing.visarga", true) };
    let layout = s.get("layout").and_then(Value::as_str).unwrap_or("phonetic");
    let safe = layout.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-');
    let mim = dir.as_ref().filter(|_| safe).and_then(|d| std::fs::read_to_string(d.join("layouts").join(format!("{layout}.mim"))).ok());
    let scheme = match layout {
        _ if mim.is_some() => Scheme::Khipro(std::sync::Arc::new(InputMethod::parse(mim.as_deref().unwrap_or_default()))),
        "phonetic" => Scheme::Phonetic(options),
        id => BUILT_IN
            .iter()
            .find(|(i, _)| *i == id)
            .and_then(|(_, t)| Layout::from_json(t).ok())
            .or_else(|| {
                let ok = id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-');
                dir.as_ref().filter(|_| ok).and_then(|d| std::fs::read_to_string(d.join("layouts").join(format!("{id}.json"))).ok()).and_then(|t| Layout::from_json(&t).ok())
            })
            .map(Scheme::Fixed)
            .unwrap_or(Scheme::Phonetic(options)),
    };
    let hk = s.get("hotkeys.toggle");
    let toggle = match hk.and_then(|h| h.get("vk")).and_then(Value::as_u64) {
        Some(vk) => {
            let f = |k: &str| hk.and_then(|h| h.get(k)).and_then(Value::as_bool).unwrap_or(false);
            Hotkey { vk: vk as u16, ctrl: f("ctrl"), alt: f("alt"), shift: f("shift") }
        }
        None => Hotkey { vk: 0x7B, ctrl: false, alt: false, shift: false },
    };
    Loaded { scheme, toggle }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(s: &str) -> Vec<Key> {
        s.chars().map(|c| key(if c.is_ascii_alphabetic() { c.to_ascii_uppercase() as u16 } else { c as u16 }, c.is_ascii_uppercase(), false, false).unwrap()).collect()
    }

    #[test]
    fn phonetic_words_compose_and_backspace_exactly() {
        let mut c = Composer::new(Scheme::Phonetic(Options::default()));
        for k in keys("amar") {
            c.push(k);
        }
        assert_eq!(c.text(), "আমার");
        assert_eq!(c.pop(), "আমা");
        assert_eq!(c.with(keys("r")[0]).as_deref(), Some("আমার"));
    }

    #[test]
    fn digits_are_not_part_of_a_word_unless_bangla_digits() {
        let c = Composer::new(Scheme::Phonetic(Options { bangla_digits: false, dari: true, visarga: true }));
        assert_eq!(c.with(keys("1")[0]), None);
        let c = Composer::new(Scheme::Phonetic(Options { bangla_digits: true, dari: true, visarga: true }));
        assert_eq!(c.with(keys("1")[0]).as_deref(), Some("১"));
    }

    #[test]
    fn unijoy_and_khipro_compose() {
        let uni = Layout::from_json(include_str!("../../assets/layouts/unijoy/unijoy.json")).unwrap();
        let mut c = Composer::new(Scheme::Fixed(uni));
        for k in keys("gfmd") {
            c.push(k);
        }
        assert_eq!(c.text(), "আমি");
        let im = std::sync::Arc::new(InputMethod::parse(include_str!("../../crates/likhon-engine/src/mit/bn-khipro-classic.mim")));
        let mut c = Composer::new(Scheme::Khipro(im.clone()));
        for k in keys("ami") {
            c.push(k);
        }
        assert_eq!(c.text(), im.convert("ami"));
    }

    #[test]
    fn settings_pick_the_layout_and_toggle() {
        let dir = std::env::temp_dir().join(format!("likhon-tsf-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(dir.join("layouts")).unwrap();
        std::fs::write(dir.join("layouts").join("unijoy.json"), include_str!("../../assets/layouts/unijoy/unijoy.json")).unwrap();
        std::fs::write(dir.join("layouts").join("khipro.mim"), include_str!("../../crates/likhon-engine/src/mit/bn-khipro-classic.mim")).unwrap();
        std::fs::write(dir.join("settings.json"), r#"{"layout":"unijoy","hotkeys.toggle":{"vk":32,"ctrl":true,"alt":false,"shift":false,"label":"Ctrl + Space"}}"#).unwrap();
        let l = load(&[dir.clone()]);
        assert!(matches!(l.scheme, Scheme::Fixed(ref f) if f.id == "unijoy"), "an added layout");
        std::fs::write(dir.join("settings.json"), r#"{"layout":"khipro"}"#).unwrap();
        assert!(matches!(load(&[dir.clone()]).scheme, Scheme::Khipro(_)), "an added input method");
        std::fs::write(dir.join("settings.json"), r#"{"layout":"unijoy","hotkeys.toggle":{"vk":32,"ctrl":true,"alt":false,"shift":false,"label":"Ctrl + Space"}}"#).unwrap();
        let l = load(&[dir.clone()]);
        assert_eq!(l.toggle, Hotkey { vk: 32, ctrl: true, alt: false, shift: false });
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(load(&[]).scheme, Scheme::Phonetic(_)));
    }
}
