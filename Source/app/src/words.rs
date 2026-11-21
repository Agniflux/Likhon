//! The word being typed, and what happens when it ends: shortcuts and autocorrect (v2 §7, Shortcuts and Autocorrect).
//!
//! The agent keeps the keys typed since the word began and the text it put on screen for them. When the word ends
//! (Space, or Tab for shortcuts) the word is looked up: a shortcut by the keys typed (";addr"), autocorrect by the
//! Bangla on screen or by the keys, then the dictionary's built-in phonetic list (a Latin word and the phonetic
//! spelling it becomes, e.g. "account" → অ্যাকাউন্ট), which applies only to a word typed in Phonetic. The lists live in
//! memory, loaded from user.sqlite and the dictionary, so the keyboard hook never waits on a database. Nothing typed is
//! stored: the buffer is dropped when the word ends.

use likhon_engine::Action;
use likhon_store::{List, UserData};
use std::collections::HashMap;

/// The current word: keys as typed (US characters, Shift applied) and the text on screen, in UTF-16 units.
#[derive(Default)]
pub struct Word {
    keys: String,
    screen: Vec<u16>,
}

/// The longest word kept; anything longer is not a word (pasted text, a held key).
const MAX_WORD: usize = 64;

impl Word {
    /// Records one key and what the engine did with it.
    pub fn record(&mut self, typed: char, action: &Action) {
        if self.keys.len() >= MAX_WORD {
            self.clear();
            return;
        }
        self.keys.push(typed);
        match action {
            Action::PassThrough => {
                let mut b = [0u16; 2];
                self.screen.extend_from_slice(typed.encode_utf16(&mut b));
            }
            Action::Replace { backspaces, output } => {
                self.screen.truncate(self.screen.len().saturating_sub(*backspaces));
                self.screen.extend(output.encode_utf16());
            }
        }
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.screen.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn keys(&self) -> &str {
        &self.keys
    }

    pub fn text(&self) -> String {
        String::from_utf16_lossy(&self.screen)
    }

    /// How many characters (UTF-16 units) to erase to replace the word.
    pub fn len(&self) -> usize {
        self.screen.len()
    }
}

/// Shortcuts and autocorrect, from user.sqlite, and the dictionary's built-in autocorrect (already in Bangla).
#[derive(Default)]
pub struct Lists {
    shortcuts: HashMap<String, String>,
    autocorrect: HashMap<String, String>,
    built_in: HashMap<String, String>,
}

impl Lists {
    pub fn load() -> Lists {
        let Ok(db) = UserData::open(&likhon_store::user_db(&crate::settings::dir().0)) else { return Lists::default() };
        let map = |list| db.all(list).into_iter().map(|e| (e.key, e.value)).collect();
        let built_in = crate::settings::dictionary_path()
            .and_then(|p| likhon_store::Dictionary::open(&p).ok())
            .map(|d| d.autocorrect().into_iter().map(|(roman, spelling)| (roman, likhon_engine::phonetic(&spelling))).collect())
            .unwrap_or_default();
        Lists { shortcuts: map(List::Shortcuts), autocorrect: map(List::Autocorrect), built_in }
    }

    /// What the word becomes when it ends, if anything: a shortcut first, then autocorrect.
    pub fn replacement(&self, word: &Word, shortcuts: bool, autocorrect: bool) -> Option<&str> {
        if word.is_empty() {
            return None;
        }
        if shortcuts {
            if let Some(v) = self.shortcuts.get(word.keys()) {
                return Some(v);
            }
        }
        if autocorrect {
            let text = word.text();
            if let Some(v) = self.autocorrect.get(&text).or_else(|| self.autocorrect.get(word.keys())) {
                return Some(v);
            }
            // Built-in: only for a word typed in Phonetic (what is on screen is the keys' phonetic spelling).
            if let Some(v) = self.built_in.get(word.keys()) {
                if v != &text && text == likhon_engine::phonetic(word.keys()) {
                    return Some(v);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(keys: &str, engine: &mut likhon_engine::Phonetic) -> Word {
        let mut w = Word::default();
        for c in keys.chars() {
            let a = engine.key(c);
            w.record(c, &a);
        }
        w
    }

    #[test]
    fn the_word_follows_the_screen() {
        let mut e = likhon_engine::Phonetic::new();
        let w = word("ami", &mut e);
        assert_eq!((w.keys(), w.text().as_str(), w.len()), ("ami", "আমি", 3));
    }

    #[test]
    fn shortcuts_by_keys_then_autocorrect_by_text() {
        let mut lists = Lists::default();
        lists.shortcuts.insert(";ty".into(), "ধন্যবাদ".into());
        lists.autocorrect.insert("আমি".into(), "আমিই".into());
        let mut e = likhon_engine::Phonetic::new();
        assert_eq!(lists.replacement(&word(";ty", &mut e), true, true), Some("ধন্যবাদ"));
        e.reset();
        assert_eq!(lists.replacement(&word(";ty", &mut e), false, true), None, "shortcuts off");
        e.reset();
        assert_eq!(lists.replacement(&word("ami", &mut e), true, true), Some("আমিই"));
        e.reset();
        assert_eq!(lists.replacement(&word("ami", &mut e), true, false), None, "autocorrect off");
    }

    #[test]
    fn built_in_autocorrect_only_for_phonetic_words() {
        let mut lists = Lists::default();
        lists.built_in.insert("account".into(), likhon_engine::phonetic("oZakaunT"));
        let mut e = likhon_engine::Phonetic::new();
        assert_eq!(lists.replacement(&word("account", &mut e), true, true), Some("অ্যাকাউন্ট"));
        // The same keys typed in a fixed layout put something else on screen: left alone.
        let mut w = Word::default();
        for c in "account".chars() {
            w.record(c, &Action::Replace { backspaces: 0, output: "ক".into() });
        }
        assert_eq!(lists.replacement(&w, true, true), None);
        // The person's own list comes first.
        lists.autocorrect.insert("account".into(), "হিসাব".into());
        e.reset();
        assert_eq!(lists.replacement(&word("account", &mut e), true, true), Some("হিসাব"));
    }
}
