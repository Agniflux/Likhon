// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! A session for input methods that show a word being typed (a "preedit") and commit it when it ends: the Linux
//! IBus engine and the macOS input method. Windows does not use this (it edits the screen in place).
//!
//! The engine edits the current word with "erase n, type text" steps (`likhon_engine::Action`). A session keeps
//! that word in a buffer, so the host only has to show `preedit` and insert `commit`. With a dictionary loaded it
//! also offers the same suggestion rows as Windows (likhon-store's `rank`), one of them selected. No network, and
//! nothing typed is stored: the buffer is the one word being typed, and it is cleared when the word ends.

use likhon_engine::fixed::{Fixed, KeyPress, Layout};
use likhon_engine::{Action, Options, Phonetic};
use likhon_store::rank::{self, Candidate};
use likhon_store::{Dictionary, UserData};
use std::collections::HashMap;
use std::ffi::{c_char, c_int, CString};
use std::path::Path;

/// How many suggestion rows (Windows' default).
const ROWS: usize = 5;

/// The built-in layouts besides Phonetic (the same files likhon.exe embeds).
const PROBHAT: &str = include_str!("../../../assets/layouts/avro/probhat.json");
const JATIYA: &str = include_str!("../../../assets/layouts/avro/jatiya.json");

/// What the typing is done with: phonetic rules, or a fixed layout.
enum Engine {
    Phonetic(Phonetic),
    Fixed(Fixed),
}

impl Engine {
    fn new(layout: &str, options: Options) -> Engine {
        let file = match layout {
            "probhat" => Some(PROBHAT),
            "jatiya" => Some(JATIYA),
            _ => None,
        };
        match file.and_then(|f| Layout::from_json(f).ok()) {
            Some(l) => Engine::Fixed(Fixed::new(l)),
            None => Engine::Phonetic(Phonetic::with_options(options)),
        }
    }

    fn is_phonetic(&self) -> bool {
        matches!(self, Engine::Phonetic(_))
    }

    fn reset(&mut self) {
        match self {
            Engine::Phonetic(p) => p.reset(),
            Engine::Fixed(f) => f.reset(),
        }
    }

    fn key(&mut self, ch: char) -> Action {
        match self {
            Engine::Phonetic(p) => p.key(ch),
            Engine::Fixed(f) => {
                let (key, shift) = unshift(ch);
                f.key(KeyPress { key, shift, altgr: false })
            }
        }
    }
}

/// The key a typed character comes from on a US keyboard, and whether Shift was held (fixed layouts are typed by
/// key position, not by the character).
fn unshift(ch: char) -> (char, bool) {
    const PAIRS: &str = r#"!1@2#3$4%5^6&7*8(9)0_-+={[}]|\:;"'<,>.?/~`"#;
    let mut it = PAIRS.chars();
    while let (Some(shifted), Some(base)) = (it.next(), it.next()) {
        if ch == shifted {
            return (base, true);
        }
    }
    if ch.is_ascii_uppercase() {
        return (ch.to_ascii_lowercase(), true);
    }
    (ch, false)
}

/// Typing settings, the same names as Windows' settings.json (`typing.*`, `layout`).
#[derive(Clone, Debug)]
pub struct Settings {
    pub layout: String,
    pub suggestions: bool,
    pub autocorrect: bool,
    pub number_pick: bool,
    pub learn: bool,
    pub prefer_dict: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { layout: "phonetic".into(), suggestions: true, autocorrect: true, number_pick: false, learn: true, prefer_dict: true }
    }
}

/// What the host must do after a key.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Step {
    /// The key was used by Likhon: do not pass it on.
    pub consumed: bool,
    /// Text to insert now (a finished word, plus the key that ended it when that key is text).
    pub commit: String,
    /// The word being typed, to show underlined. Empty: no preedit.
    pub preedit: String,
}

pub struct Session {
    engine: Engine,
    options: Options,
    settings: Settings,
    /// The field is a password or PIN box: no suggestions, nothing learned.
    private: bool,
    user: Option<UserData>,
    /// The word as Likhon shows it now (Bangla).
    word: Vec<char>,
    /// The Latin keys that made it, so Backspace can take one letter off.
    keys: Vec<char>,
    /// The preedit is a dari from `.`, not a word.
    dari: bool,
    dict: Option<Dictionary>,
    /// The dictionary's autocorrect list: what was typed in Latin letters, and the Bangla to give instead.
    auto: HashMap<String, String>,
    rows: Vec<Candidate>,
    selected: usize,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        let options = Options::default();
        let settings = Settings::default();
        Self {
            engine: Engine::new(&settings.layout, options),
            options,
            settings,
            private: false,
            user: None,
            word: Vec::new(),
            keys: Vec::new(),
            dari: false,
            dict: None,
            auto: HashMap::new(),
            rows: Vec::new(),
            selected: 0,
        }
    }

    /// Opens the dictionary (a read-only SQLite file) for suggestions and autocorrect.
    pub fn load_dictionary(&mut self, path: &Path) -> Result<(), String> {
        let dict = Dictionary::open(path)?;
        self.auto = dict.autocorrect().into_iter().map(|(roman, spelling)| (roman, likhon_engine::phonetic(&spelling))).collect();
        self.dict = Some(dict);
        self.refresh_rows();
        Ok(())
    }

    pub fn set_options(&mut self, options: Options) {
        self.options = options;
        self.engine = Engine::new(&self.settings.layout, options);
        self.dari = false;
        self.word.clear();
        self.keys.clear();
        self.refresh_rows();
    }

    /// Applies a settings file (Windows' settings.json names: `layout`, `typing.bangla_digits`, `typing.dari`,
    /// `typing.visarga`, `typing.autocorrect`, `typing.suggestions`, `typing.number_pick`, `typing.learn`,
    /// `typing.preselect`). Missing keys keep the defaults; a file that is not JSON changes nothing. Ends the word.
    pub fn apply_settings(&mut self, json: &str) -> bool {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(json.trim_start_matches('\u{feff}')) else { return false };
        let flag = |k: &str, d: bool| v.get(k).and_then(|x| x.as_bool()).unwrap_or(d);
        let d = Settings::default();
        self.settings = Settings {
            layout: v.get("layout").and_then(|x| x.as_str()).unwrap_or(&d.layout).to_string(),
            suggestions: flag("typing.suggestions", d.suggestions),
            autocorrect: flag("typing.autocorrect", d.autocorrect),
            number_pick: flag("typing.number_pick", d.number_pick),
            learn: flag("typing.learn", d.learn),
            prefer_dict: v.get("typing.preselect").and_then(|x| x.as_str()) != Some("plain"),
        };
        self.set_options(Options {
            bangla_digits: flag("typing.bangla_digits", false),
            dari: flag("typing.dari", true),
            visarga: flag("typing.visarga", true),
        });
        true
    }

    /// Opens the person's own words (user.sqlite) for "my words" rows and for learning. Without it nothing is learned.
    pub fn open_user_data(&mut self, path: &Path) -> Result<(), String> {
        self.user = Some(UserData::open(path)?);
        Ok(())
    }

    /// A password or PIN field: no suggestions, nothing learned. The word being typed ends.
    pub fn set_private(&mut self, private: bool) {
        self.private = private;
        self.rows.clear();
    }

    fn learn(&self, word: &str) {
        if self.settings.learn && !self.private {
            if let Some(u) = &self.user {
                u.learn(word);
            }
        }
    }

    fn preedit(&self) -> String {
        self.word.iter().collect()
    }

    /// The suggestion rows for the word being typed, as text.
    pub fn candidates(&self) -> Vec<&str> {
        self.rows.iter().map(|c| c.word.as_str()).collect()
    }

    /// Which row is selected (meaningful only when there are rows).
    pub fn selected(&self) -> usize {
        self.selected
    }

    fn refresh_rows(&mut self) {
        self.rows.clear();
        self.selected = 0;
        if self.dari || self.word.is_empty() || self.dict.is_none() || self.private || !self.settings.suggestions || !self.engine.is_phonetic() {
            return;
        }
        let text = self.preedit();
        let typed: String = self.keys.iter().collect();
        let auto = if self.settings.autocorrect { self.auto.get(&typed.to_lowercase()).map(String::as_str) } else { None };
        let mine = self.user.as_ref().map(|u| u.words_starting_with(&text, ROWS)).unwrap_or_default();
        self.rows = rank::candidates(&text, auto, &mine, self.dict.as_ref(), ROWS);
        self.selected = rank::preselect(&self.rows, &text, self.dict.as_ref(), self.settings.prefer_dict);
    }

    /// The word Space or Enter would type now: the selected row, or the word as it is.
    fn chosen(&self) -> String {
        self.rows.get(self.selected).map(|c| c.word.clone()).unwrap_or_else(|| self.preedit())
    }

    /// Ends the word: its text is returned for committing, and the engine starts fresh.
    fn finish(&mut self) -> String {
        let text = self.preedit();
        self.dari = false;
        self.word.clear();
        self.keys.clear();
        self.rows.clear();
        self.selected = 0;
        self.engine.reset();
        text
    }

    /// A typed character (already shifted: `A` for Shift+a). The host passes only keys that are plain text;
    /// shortcuts (Ctrl, Alt, Super) end the word through `reset` instead.
    pub fn key(&mut self, ch: char) -> Step {
        // A dari from the last `.` is still shown as a preedit (so a second `.` can turn it into a full stop).
        // Any other key commits it first.
        // Number keys pick a suggestion row while a word is being typed (Typing > Number keys, off by default).
        if self.settings.number_pick && ('1'..='9').contains(&ch) && !self.word.is_empty() && !self.dari {
            let index = ch as usize - '1' as usize;
            if index < self.rows.len() {
                return self.pick(index);
            }
        }
        let mut commit = String::new();
        if self.dari && ch != '.' {
            commit = self.preedit();
            self.word.clear();
            self.dari = false;
        }
        // Space types the selected suggestion (a correction, or the word as typed) before the space itself.
        let space_word = if ch == ' ' && !self.dari { Some(self.chosen()) } else { None };
        match self.engine.key(ch) {
            Action::PassThrough => {
                let typed = self.finish();
                let word = space_word.filter(|_| !typed.is_empty()).unwrap_or(typed);
                self.learn(&word);
                commit.push_str(&word);
                if ch.is_control() {
                    return Step { consumed: false, commit, preedit: String::new() };
                }
                commit.push(ch);
                Step { consumed: true, commit, preedit: String::new() }
            }
            Action::Replace { backspaces, output } if ch == '.' && self.engine.is_phonetic() => {
                self.rows.clear();
                if backspaces == 0 {
                    // The word before the dot is done; the dari stays in the preedit.
                    commit.push_str(&self.preedit());
                    self.word = output.chars().collect();
                    self.keys.clear();
                    self.dari = true;
                    Step { consumed: true, commit, preedit: self.preedit() }
                } else {
                    // "..": the dari just shown becomes a full stop, and that ends it.
                    self.word.clear();
                    self.keys.clear();
                    self.dari = false;
                    self.engine.reset();
                    commit.push_str(&output);
                    Step { consumed: true, commit, preedit: String::new() }
                }
            }
            Action::Replace { backspaces, output } => {
                for _ in 0..backspaces {
                    self.word.pop();
                }
                self.word.extend(output.chars());
                self.keys.push(ch);
                self.refresh_rows();
                Step { consumed: true, commit, preedit: self.preedit() }
            }
        }
    }

    /// Backspace. With a word being typed, takes one letter off it; otherwise the host deletes text itself.
    pub fn backspace(&mut self) -> Step {
        if self.dari {
            self.word.clear();
            self.dari = false;
            self.engine.reset();
            return Step { consumed: true, ..Step::default() };
        }
        if self.keys.is_empty() {
            return Step::default();
        }
        self.keys.pop();
        let keys = std::mem::take(&mut self.keys);
        self.word.clear();
        self.engine = Engine::new(&self.settings.layout, self.options);
        for ch in keys {
            if let Action::Replace { backspaces, output } = self.engine.key(ch) {
                for _ in 0..backspaces {
                    self.word.pop();
                }
                self.word.extend(output.chars());
            }
            self.keys.push(ch);
        }
        self.refresh_rows();
        Step { consumed: true, commit: String::new(), preedit: self.preedit() }
    }

    /// Up or Down: moves the selected row. Not used (the host handles the key) when there is no list.
    pub fn move_selection(&mut self, down: bool) -> Step {
        let n = self.rows.len();
        if n == 0 {
            return Step::default();
        }
        self.selected = if down { (self.selected + 1) % n } else { (self.selected + n - 1) % n };
        Step { consumed: true, commit: String::new(), preedit: self.preedit() }
    }

    /// Enter: types the selected row and ends the word (the Enter itself is not passed on).
    pub fn accept(&mut self) -> Step {
        if self.word.is_empty() || self.dari {
            return self.reset();
        }
        let chosen = self.chosen();
        self.finish();
        self.learn(&chosen);
        Step { consumed: true, commit: chosen, preedit: String::new() }
    }

    /// A number key: row `index` (0-based) is typed, with a space after it. False when there is no such row.
    pub fn pick(&mut self, index: usize) -> Step {
        let Some(word) = self.rows.get(index).map(|c| c.word.clone()) else { return Step::default() };
        self.finish();
        self.learn(&word);
        Step { consumed: true, commit: word + " ", preedit: String::new() }
    }

    /// Escape: drop the word without typing it.
    pub fn cancel(&mut self) -> Step {
        let had = !self.word.is_empty();
        self.word.clear();
        self.keys.clear();
        self.rows.clear();
        self.dari = false;
        self.engine.reset();
        Step { consumed: had, commit: String::new(), preedit: String::new() }
    }

    /// Focus left, the caret moved, a shortcut was pressed: the word ends and is committed as it is.
    pub fn reset(&mut self) -> Step {
        let commit = self.finish();
        Step { consumed: false, commit, preedit: String::new() }
    }
}

// ---- C interface -------------------------------------------------------------------------------------------------

/// Opaque to C. Owns the strings it hands out, which stay valid until the next call on the same session.
pub struct LikhonSession {
    inner: Session,
    commit: CString,
    preedit: CString,
    rows: Vec<CString>,
}

fn cstring(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_default()
}

impl LikhonSession {
    /// # Safety
    /// `commit` and `preedit` are valid pointers or null.
    unsafe fn put(&mut self, step: Step, commit: *mut *const c_char, preedit: *mut *const c_char) -> c_int {
        self.commit = cstring(&step.commit);
        self.preedit = cstring(&step.preedit);
        self.rows = self.inner.candidates().into_iter().map(cstring).collect();
        if !commit.is_null() {
            *commit = self.commit.as_ptr();
        }
        if !preedit.is_null() {
            *preedit = self.preedit.as_ptr();
        }
        step.consumed as c_int
    }
}

/// The version of Likhon, a static string.
#[no_mangle]
pub extern "C" fn likhon_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

#[no_mangle]
pub extern "C" fn likhon_session_new() -> *mut LikhonSession {
    Box::into_raw(Box::new(LikhonSession { inner: Session::new(), commit: CString::default(), preedit: CString::default(), rows: Vec::new() }))
}

/// # Safety
/// `s` is from `likhon_session_new` and not used again.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_free(s: *mut LikhonSession) {
    if !s.is_null() {
        drop(Box::from_raw(s));
    }
}

/// Typing options: Bangla digits, full stop as dari, colon as visarga (each 0 or 1). Ends the current word.
///
/// # Safety
/// `s` is a live session.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_set_options(s: *mut LikhonSession, bangla_digits: c_int, dari: c_int, visarga: c_int) {
    if let Some(s) = s.as_mut() {
        s.inner.set_options(Options { bangla_digits: bangla_digits != 0, dari: dari != 0, visarga: visarga != 0 });
        s.rows.clear();
    }
}

/// Opens the dictionary file (UTF-8 path) for suggestions. Returns 1 on success, 0 when the file is missing or not a
/// Likhon dictionary; typing works either way, just without suggestions.
///
/// # Safety
/// `s` is a live session; `path` is a NUL-terminated string or null.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_load_dictionary(s: *mut LikhonSession, path: *const c_char) -> c_int {
    let (Some(s), false) = (s.as_mut(), path.is_null()) else { return 0 };
    let Ok(path) = std::ffi::CStr::from_ptr(path).to_str() else { return 0 };
    s.inner.load_dictionary(Path::new(path)).is_ok() as c_int
}

/// Applies a settings file's text (UTF-8 JSON, Windows' settings.json names). Returns 1 on success, 0 when it is not
/// JSON (nothing changes). Ends the word being typed.
///
/// # Safety
/// `s` is a live session; `json` is a NUL-terminated string or null.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_apply_settings(s: *mut LikhonSession, json: *const c_char) -> c_int {
    let (Some(s), false) = (s.as_mut(), json.is_null()) else { return 0 };
    let Ok(json) = std::ffi::CStr::from_ptr(json).to_str() else { return 0 };
    let ok = s.inner.apply_settings(json);
    s.rows.clear();
    ok as c_int
}

/// Opens the person's own words (user.sqlite, created if missing) for "my words" rows and learning.
///
/// # Safety
/// `s` is a live session; `path` is a NUL-terminated string or null.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_open_user_data(s: *mut LikhonSession, path: *const c_char) -> c_int {
    let (Some(s), false) = (s.as_mut(), path.is_null()) else { return 0 };
    let Ok(path) = std::ffi::CStr::from_ptr(path).to_str() else { return 0 };
    s.inner.open_user_data(Path::new(path)).is_ok() as c_int
}

/// 1 for a password or PIN field (no suggestions, nothing learned), 0 otherwise.
///
/// # Safety
/// `s` is a live session.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_set_private(s: *mut LikhonSession, private: c_int) {
    if let Some(s) = s.as_mut() {
        s.inner.set_private(private != 0);
        s.rows.clear();
    }
}

/// A typed character, as a Unicode scalar value. Returns 1 when Likhon used the key (do not pass it on), 0 when the
/// host should handle it. `commit` is text to insert now and `preedit` the word being typed (both UTF-8, possibly
/// empty, valid until the next call on this session).
///
/// # Safety
/// `s` is a live session; `commit` and `preedit` are valid pointers or null.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_key(s: *mut LikhonSession, ch: u32, commit: *mut *const c_char, preedit: *mut *const c_char) -> c_int {
    let Some(s) = s.as_mut() else { return 0 };
    let Some(ch) = char::from_u32(ch) else { return 0 };
    let step = s.inner.key(ch);
    s.put(step, commit, preedit)
}

/// # Safety
/// As `likhon_session_key`.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_backspace(s: *mut LikhonSession, commit: *mut *const c_char, preedit: *mut *const c_char) -> c_int {
    let Some(s) = s.as_mut() else { return 0 };
    let step = s.inner.backspace();
    s.put(step, commit, preedit)
}

/// Up (`down` = 0) or Down (1) in the suggestion list. Returns 0 when there is no list.
///
/// # Safety
/// As `likhon_session_key`.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_move(s: *mut LikhonSession, down: c_int, commit: *mut *const c_char, preedit: *mut *const c_char) -> c_int {
    let Some(s) = s.as_mut() else { return 0 };
    let step = s.inner.move_selection(down != 0);
    s.put(step, commit, preedit)
}

/// Enter: types the selected suggestion and ends the word. Returns 1 when it did (swallow the Enter).
///
/// # Safety
/// As `likhon_session_key`.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_accept(s: *mut LikhonSession, commit: *mut *const c_char, preedit: *mut *const c_char) -> c_int {
    let Some(s) = s.as_mut() else { return 0 };
    let step = s.inner.accept();
    s.put(step, commit, preedit)
}

/// A number key: suggestion row `index` (0-based) is typed with a space after it. Returns 0 when there is no such row.
///
/// # Safety
/// As `likhon_session_key`.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_pick(s: *mut LikhonSession, index: c_int, commit: *mut *const c_char, preedit: *mut *const c_char) -> c_int {
    let Some(s) = s.as_mut() else { return 0 };
    let step = s.inner.pick(index.max(0) as usize);
    s.put(step, commit, preedit)
}

/// # Safety
/// As `likhon_session_key`.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_cancel(s: *mut LikhonSession, commit: *mut *const c_char, preedit: *mut *const c_char) -> c_int {
    let Some(s) = s.as_mut() else { return 0 };
    let step = s.inner.cancel();
    s.put(step, commit, preedit)
}

/// The word ends (focus out, a shortcut, the caret moved): `commit` holds the word as shown.
///
/// # Safety
/// As `likhon_session_key`.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_reset(s: *mut LikhonSession, commit: *mut *const c_char, preedit: *mut *const c_char) -> c_int {
    let Some(s) = s.as_mut() else { return 0 };
    let step = s.inner.reset();
    s.put(step, commit, preedit)
}

/// How many suggestion rows there are after the last call (0 when none or no dictionary).
///
/// # Safety
/// `s` is a live session.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_row_count(s: *const LikhonSession) -> c_int {
    s.as_ref().map_or(0, |s| s.rows.len() as c_int)
}

/// Row `i` as UTF-8 (valid until the next call on this session), or null.
///
/// # Safety
/// `s` is a live session.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_row(s: *const LikhonSession, i: c_int) -> *const c_char {
    s.as_ref().and_then(|s| s.rows.get(i.max(0) as usize)).map_or(std::ptr::null(), |c| c.as_ptr())
}

/// The selected row.
///
/// # Safety
/// `s` is a live session.
#[no_mangle]
pub unsafe extern "C" fn likhon_session_selected(s: *const LikhonSession) -> c_int {
    s.as_ref().map_or(0, |s| s.inner.selected() as c_int)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Types `keys` and returns what the host would hold: committed text plus the preedit.
    fn type_all(keys: &str) -> String {
        let mut s = Session::new();
        let mut doc = String::new();
        let mut pre = String::new();
        for ch in keys.chars() {
            let step = s.key(ch);
            doc.push_str(&step.commit);
            pre = step.preedit;
        }
        doc + &pre
    }

    fn dict_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/dict/mpl/words.sqlite")
    }

    fn with_dictionary() -> Session {
        let mut s = Session::new();
        s.load_dictionary(&dict_path()).expect("dictionary");
        s
    }

    #[test]
    fn ami_shows_in_the_preedit_then_commits_with_the_space() {
        let mut s = Session::new();
        let mut last = Step::default();
        for ch in "ami".chars() {
            last = s.key(ch);
        }
        assert_eq!(last.preedit, "আমি");
        assert_eq!(last.commit, "");
        let space = s.key(' ');
        assert!(space.consumed);
        assert_eq!(space.commit, "আমি ");
        assert_eq!(space.preedit, "");
    }

    #[test]
    fn sentences_match_the_converter() {
        for s in ["amar sOnar bangla", "ami banglay gan gai", "kemon achho", "shikkha bangladesh"] {
            let whole: String = s.split(' ').map(likhon_engine::phonetic).collect::<Vec<_>>().join(" ");
            assert_eq!(type_all(s), whole, "{s}");
        }
        assert_eq!(type_all("amar sOnar bangla"), "আমার সোনার বাংলা");
    }

    #[test]
    fn dot_becomes_a_dari_that_ends_the_word() {
        assert_eq!(type_all("ami."), "আমি।");
        assert_eq!(type_all("ami. tumi."), "আমি। তুমি।");
    }

    #[test]
    fn backspace_takes_one_letter_off_the_word() {
        let mut s = Session::new();
        for ch in "banglaa".chars() {
            s.key(ch);
        }
        let b = s.backspace();
        assert!(b.consumed);
        assert_eq!(b.preedit, "বাংলা");
        // With no word, the host deletes text itself.
        s.cancel();
        assert!(!s.backspace().consumed);
    }

    #[test]
    fn escape_drops_the_word_and_reset_commits_it() {
        let mut s = Session::new();
        for ch in "ami".chars() {
            s.key(ch);
        }
        assert_eq!(s.reset().commit, "আমি");
        for ch in "tumi".chars() {
            s.key(ch);
        }
        let c = s.cancel();
        assert!(c.consumed && c.preedit.is_empty() && c.commit.is_empty());
        assert_eq!(s.reset().commit, "");
    }

    #[test]
    fn digits_and_the_colon_follow_the_options() {
        assert_eq!(type_all("ami 12"), "আমি 12");
        let mut s = Session::new();
        s.set_options(Options { bangla_digits: true, dari: true, visarga: false });
        let mut doc = String::new();
        for ch in "ami 12:".chars() {
            doc.push_str(&s.key(ch).commit);
        }
        assert_eq!(doc, "আমি ১২:");
    }

    #[test]
    fn without_a_dictionary_there_are_no_rows() {
        let mut s = Session::new();
        for ch in "bangla".chars() {
            s.key(ch);
        }
        assert!(s.candidates().is_empty());
        assert!(!s.move_selection(true).consumed);
    }

    #[test]
    fn a_dictionary_offers_rows_with_the_typed_word_among_them() {
        let mut s = with_dictionary();
        for ch in "bangla".chars() {
            s.key(ch);
        }
        let rows = s.candidates();
        assert!(rows.len() > 1 && rows.contains(&"বাংলা"), "{rows:?}");
        assert!(s.selected() < rows.len());
    }

    #[test]
    fn space_types_the_selected_row_and_up_down_move_it() {
        let mut s = with_dictionary();
        for ch in "bangla".chars() {
            s.key(ch);
        }
        let first = s.candidates()[s.selected()].to_string();
        let moved = s.move_selection(true);
        assert!(moved.consumed);
        let second = s.candidates()[s.selected()].to_string();
        assert_ne!(first, second);
        let space = s.key(' ');
        assert_eq!(space.commit, format!("{second} "));
        assert!(s.candidates().is_empty());
    }

    #[test]
    fn enter_types_the_selected_row_without_a_space() {
        let mut s = with_dictionary();
        for ch in "bangla".chars() {
            s.key(ch);
        }
        let want = s.candidates()[s.selected()].to_string();
        let step = s.accept();
        assert!(step.consumed);
        assert_eq!(step.commit, want);
        // With no word, Enter belongs to the application.
        assert!(!s.accept().consumed);
    }

    #[test]
    fn a_number_key_picks_a_row_with_a_space() {
        let mut s = with_dictionary();
        for ch in "bangla".chars() {
            s.key(ch);
        }
        let want = format!("{} ", s.candidates()[1]);
        assert_eq!(s.pick(1).commit, want);
        assert!(!s.pick(1).consumed);
    }

    #[test]
    fn autocorrect_is_preselected() {
        let mut s = with_dictionary();
        for ch in "account".chars() {
            s.key(ch);
        }
        // The dictionary's autocorrect list turns "account" into অ্যাকাউন্ট.
        let chosen = s.candidates()[s.selected()].to_string();
        assert_eq!(chosen, likhon_engine::phonetic("oZakaunT"));
    }

    fn type_in(s: &mut Session, keys: &str) -> String {
        let mut doc = String::new();
        let mut pre = String::new();
        for ch in keys.chars() {
            let step = s.key(ch);
            doc.push_str(&step.commit);
            pre = step.preedit;
        }
        doc + &pre
    }

    #[test]
    fn settings_json_changes_the_options() {
        let mut s = Session::new();
        assert!(s.apply_settings(r#"{"typing.bangla_digits": true, "typing.visarga": false, "typing.dari": false}"#));
        assert_eq!(type_in(&mut s, "ami 12: ."), "আমি ১২: .");
        // Not JSON: nothing changes.
        assert!(!s.apply_settings("{oops"));
        assert_eq!(type_in(&mut s, "ami 1"), "আমি ১");
    }

    #[test]
    fn fixed_layouts_type_like_the_engine_does() {
        use likhon_engine::fixed::{Fixed, KeyPress, Layout};
        for (name, file) in [("probhat", PROBHAT), ("jatiya", JATIYA)] {
            let mut s = Session::new();
            assert!(s.apply_settings(&format!(r#"{{"layout": "{name}"}}"#)));
            let mut direct = Fixed::new(Layout::from_json(file).unwrap());
            let mut want = String::new();
            let mut doc = String::new();
            let mut pre = String::new();
            for ch in "kaTimR[]".chars() {
                let (key, shift) = unshift(ch);
                match direct.key(KeyPress { key, shift, altgr: false }) {
                    Action::Replace { backspaces, output } => {
                        for _ in 0..backspaces {
                            want.pop();
                        }
                        want.push_str(&output);
                    }
                    Action::PassThrough => want.push(ch),
                }
                let step = s.key(ch);
                doc.push_str(&step.commit);
                pre = step.preedit;
            }
            eprintln!("{name}: kaTimR[] -> {want}");
            assert_eq!(doc + &pre, want, "{name}");
            assert!(!want.is_empty());
        }
    }

    #[test]
    fn fixed_layouts_have_no_suggestions_and_space_ends_the_word() {
        let mut s = with_dictionary();
        s.apply_settings(r#"{"layout": "probhat"}"#);
        let typed = type_in(&mut s, "kk");
        assert!(!typed.is_empty());
        assert!(s.candidates().is_empty());
        let space = s.key(' ');
        assert!(space.commit.ends_with(' ') && space.preedit.is_empty());
    }

    #[test]
    fn number_keys_pick_only_when_switched_on() {
        let mut s = with_dictionary();
        type_in(&mut s, "bangla");
        // Off (the default): a digit ends the word and types itself.
        let r = s.key('2');
        assert!(r.commit.ends_with('2'), "{r:?}");

        let mut s = with_dictionary();
        s.apply_settings(r#"{"typing.number_pick": true}"#);
        type_in(&mut s, "bangla");
        let want = format!("{} ", s.candidates()[1]);
        assert_eq!(s.key('2').commit, want);
        // With no word being typed, the digit is just a digit.
        assert_eq!(s.key('3').commit, "3");
    }

    #[test]
    fn suggestions_and_autocorrect_can_be_switched_off() {
        let mut s = with_dictionary();
        s.apply_settings(r#"{"typing.suggestions": false}"#);
        type_in(&mut s, "bangla");
        assert!(s.candidates().is_empty());
        let mut s = with_dictionary();
        s.apply_settings(r#"{"typing.autocorrect": false}"#);
        type_in(&mut s, "account");
        let chosen = s.candidates()[s.selected()].to_string();
        assert_ne!(chosen, likhon_engine::phonetic("oZakaunT"));
    }

    fn temp_db(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("likhon-ffi-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("user.sqlite")
    }

    #[test]
    fn finished_words_are_learned_and_offered_as_my_words() {
        let db = temp_db("learn");
        let mut s = with_dictionary();
        s.open_user_data(&db).unwrap();
        // A made-up word that is in no dictionary: after it is typed once it is learned, and typing its beginning
        // again offers it as one of "my words".
        let word = likhon_engine::phonetic("zzqxz");
        assert!(!s.dict.as_ref().unwrap().contains(&word));
        type_in(&mut s, "zzqxz ");
        assert_eq!(UserData::open(&db).unwrap().learned(), vec![word.clone()]);
        let start = likhon_engine::phonetic("zzq");
        assert!(word.starts_with(&start), "test needs a prefix: {start} {word}");
        type_in(&mut s, "zzq");
        assert!(s.rows.iter().any(|c| c.word == word && c.tag == rank::Tag::Mine), "{:?}", s.rows);
        std::fs::remove_dir_all(db.parent().unwrap()).ok();
    }

    #[test]
    fn nothing_is_learned_in_a_password_field_or_when_learning_is_off() {
        let db = temp_db("private");
        let mut s = with_dictionary();
        s.open_user_data(&db).unwrap();
        s.set_private(true);
        type_in(&mut s, "ami ");
        // No suggestions in a private field either.
        type_in(&mut s, "bangla");
        assert!(s.candidates().is_empty());
        s.cancel();
        assert!(UserData::open(&db).unwrap().learned().is_empty());

        s.set_private(false);
        s.apply_settings(r#"{"typing.learn": false}"#);
        type_in(&mut s, "ami ");
        assert!(UserData::open(&db).unwrap().learned().is_empty());

        s.apply_settings(r#"{"typing.learn": true}"#);
        type_in(&mut s, "ami ");
        assert_eq!(UserData::open(&db).unwrap().learned().len(), 1);
        std::fs::remove_dir_all(db.parent().unwrap()).ok();
    }

    #[test]
    fn c_interface_round_trip() {
        unsafe {
            let s = likhon_session_new();
            let (mut commit, mut preedit): (*const c_char, *const c_char) = (std::ptr::null(), std::ptr::null());
            for ch in "ami".chars() {
                assert_eq!(likhon_session_key(s, ch as u32, &mut commit, &mut preedit), 1);
            }
            assert_eq!(std::ffi::CStr::from_ptr(preedit).to_str().unwrap(), "আমি");
            assert_eq!(likhon_session_row_count(s), 0);
            assert_eq!(likhon_session_key(s, ' ' as u32, &mut commit, &mut preedit), 1);
            assert_eq!(std::ffi::CStr::from_ptr(commit).to_str().unwrap(), "আমি ");
            likhon_session_free(s);
        }
    }

    #[test]
    fn c_interface_rows_with_a_dictionary() {
        unsafe {
            let s = likhon_session_new();
            let path = CString::new(dict_path().to_str().unwrap()).unwrap();
            assert_eq!(likhon_session_load_dictionary(s, path.as_ptr()), 1);
            assert_eq!(likhon_session_load_dictionary(s, c"/no/such/file".as_ptr()), 0);
            let (mut commit, mut preedit): (*const c_char, *const c_char) = (std::ptr::null(), std::ptr::null());
            for ch in "bangla".chars() {
                likhon_session_key(s, ch as u32, &mut commit, &mut preedit);
            }
            let n = likhon_session_row_count(s);
            assert!(n > 1);
            assert!(!likhon_session_row(s, 0).is_null());
            assert!(likhon_session_row(s, n).is_null());
            assert!(likhon_session_selected(s) < n);
            likhon_session_free(s);
        }
    }
}
