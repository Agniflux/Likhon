// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! Likhon's data in SQLite (Desktop/CLAUDE.md, section 7).
//!
//! The dictionary (owner decision 2026-10-22): a read-only SQLite file in the `dict` folder beside likhon.exe,
//! format `likhon-dictionary` version 1 (`meta`, `words`, `suffixes`, and optionally `autocorrect`; made by
//! scripts/avro/convert-dictionary.mjs, MPL-2.0). It is opened read-only and never written.
//!
//! User data (user.sqlite in the data folder): My words, autocorrect, shortcuts and word choices that the person adds or
//! imports. Every list is SQLite (owner decision 2026-10-22), none is a text file. Never store what a person types
//! except confirmed personal words from non-password fields (root CLAUDE.md, Never do); nothing is learned from typing
//! yet, because password fields cannot be told apart yet.

pub mod rank;

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::path::Path;

/// What a dictionary file says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryInfo {
    pub words: u64,
    /// The site's snapshot version ("2026.11") for a dictionary built there; None for the base file.
    pub snapshot: Option<String>,
    pub suffixes: u64,
    pub autocorrect: u64,
    pub license: String,
}

/// A read-only dictionary file.
pub struct Dictionary {
    db: Connection,
    /// The optional `more` table: added words, offered after the curated `words` (scripts/dict/build-dictionary.py).
    more: bool,
}

impl Dictionary {
    /// Opens a dictionary read-only and checks that it is a Likhon dictionary, version 1.
    pub fn open(path: &Path) -> Result<Dictionary, String> {
        let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(|e| e.to_string())?;
        let more = db.query_row("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'more'", [], |_| Ok(())).optional().ok().flatten().is_some();
        let d = Dictionary { db, more };
        if d.meta("format").as_deref() != Some("likhon-dictionary") || d.meta("version").as_deref() != Some("1") {
            return Err("not a Likhon dictionary, version 1".into());
        }
        Ok(d)
    }

    fn meta(&self, key: &str) -> Option<String> {
        self.db.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional().ok().flatten()
    }

    pub fn info(&self) -> DictionaryInfo {
        let count = |sql: &str| self.db.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap_or(0) as u64;
        DictionaryInfo {
            words: count("SELECT count(*) FROM words") + if self.more { count("SELECT count(*) FROM more") } else { 0 },
            snapshot: self.meta("snapshot"),
            suffixes: count("SELECT count(*) FROM suffixes"),
            autocorrect: if self.has_autocorrect() { count("SELECT count(*) FROM autocorrect") } else { 0 },
            license: self.meta("license").unwrap_or_default(),
        }
    }

    pub fn contains(&self, word: &str) -> bool {
        let has = |sql: &str| self.db.query_row(sql, [word], |_| Ok(())).optional().ok().flatten().is_some();
        has("SELECT 1 FROM words WHERE word = ?1") || (self.more && has("SELECT 1 FROM more WHERE word = ?1"))
    }

    /// Words that start with `prefix`, at most `limit`: the curated words first, then the added ones; within each,
    /// shorter words first (the file has no word frequencies, and the nearest whole word is usually the shortest).
    /// Each is an index range scan of at most a few hundred rows.
    pub fn starting_with(&self, prefix: &str, limit: usize) -> Vec<String> {
        const SCAN: usize = 400;
        let shortest = |sql: &str| {
            let mut v = prefix_query(&self.db, sql, prefix, SCAN);
            v.sort_by_key(|w| w.chars().count());
            v
        };
        let mut out = shortest("SELECT word FROM words WHERE word >= ?1 AND word < ?2 ORDER BY word LIMIT ?3");
        out.truncate(limit);
        if out.len() < limit && self.more {
            for w in shortest("SELECT word FROM more WHERE word >= ?1 AND word < ?2 ORDER BY word LIMIT ?3") {
                if out.len() >= limit {
                    break;
                }
                out.push(w);
            }
        }
        out
    }

    /// The Bangla ending for a typed Latin ending, e.g. "er" gives "ের".
    pub fn suffix(&self, roman: &str) -> Option<String> {
        self.db.query_row("SELECT bangla FROM suffixes WHERE roman = ?1", [roman], |r| r.get(0)).optional().ok().flatten()
    }

    fn has_autocorrect(&self) -> bool {
        self.db.query_row("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'autocorrect'", [], |_| Ok(())).optional().ok().flatten().is_some()
    }

    /// The built-in phonetic autocorrect list: a word typed in Latin letters and the phonetic spelling to type
    /// instead, e.g. ("account", "oZakaunT"). Empty for a dictionary without the table.
    pub fn autocorrect(&self) -> Vec<(String, String)> {
        if !self.has_autocorrect() {
            return Vec::new();
        }
        let Ok(mut q) = self.db.prepare("SELECT roman, phonetic FROM autocorrect") else { return Vec::new() };
        q.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map(|rows| rows.flatten().collect()).unwrap_or_default()
    }
}

fn prefix_query(db: &Connection, sql: &str, prefix: &str, limit: usize) -> Vec<String> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let end = format!("{prefix}\u{10FFFF}");
    let Ok(mut q) = db.prepare_cached(sql) else { return Vec::new() };
    q.query_map(rusqlite::params![prefix, end, limit as i64], |r| r.get(0)).map(|rows| rows.flatten().collect()).unwrap_or_default()
}

// ---- User data -------------------------------------------------------------------------------------

/// Which personal list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    /// My words: Bangla words the person added. First in suggestions.
    Words,
    /// Autocorrect: a typed word (Bangla, or the Latin letters typed) and what it becomes.
    Autocorrect,
    /// Shortcuts: a short code typed in Latin letters (";addr") and the text it expands to.
    Shortcuts,
    /// Word choices: the Latin letters typed and the suggestion the person wants picked first for them. Filled by
    /// import (e.g. a list kept by another keyboard: "ami আমি" per line), never by watching typing.
    Choices,
}

impl List {
    pub fn parse(s: &str) -> Option<List> {
        Some(match s {
            "words" => List::Words,
            "autocorrect" => List::Autocorrect,
            "shortcuts" => List::Shortcuts,
            "choices" => List::Choices,
            _ => return None,
        })
    }

    fn table(self) -> &'static str {
        match self {
            List::Words => "words",
            List::Autocorrect => "autocorrect",
            List::Shortcuts => "shortcuts",
            List::Choices => "choices",
        }
    }
}

/// One entry: `key` is the word, the typed word or the code; `value` is empty for My words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub key: String,
    pub value: String,
    /// Seconds since 1970 when it was added.
    pub added: i64,
}

/// The longest key and value accepted (a shortcut can expand to an address or a signature).
pub const MAX_KEY: usize = 60;
pub const MAX_VALUE: usize = 500;
/// The most entries one list holds (guards an import of a huge file).
pub const MAX_ENTRIES: usize = 50_000;

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// The person's own lists in `dir`: user.sqlite. Copies from before 2026-10-24 kept it as user.db: that file (and its
/// -wal and -shm files) is renamed the first time. If the rename cannot happen yet (another Likhon process has the file
/// open), the old name is used until it can.
pub fn user_db(dir: &Path) -> std::path::PathBuf {
    let new = dir.join("user.sqlite");
    let old = dir.join("user.db");
    if new.exists() || !old.exists() {
        return new;
    }
    if std::fs::rename(&old, &new).is_err() {
        return old;
    }
    for ext in ["-wal", "-shm"] {
        let _ = std::fs::rename(dir.join(format!("user.db{ext}")), dir.join(format!("user.sqlite{ext}")));
    }
    new
}

/// user.sqlite in the data folder: the person's own lists. Nothing typed is stored except what is added here on purpose.
pub struct UserData {
    db: Connection,
}

impl UserData {
    pub fn open(path: &Path) -> Result<UserData, String> {
        let db = Connection::open(path).map_err(|e| e.to_string())?;
        db.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA busy_timeout = 2000;
             CREATE TABLE IF NOT EXISTS words (key TEXT PRIMARY KEY, value TEXT NOT NULL DEFAULT '', added INTEGER NOT NULL) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS autocorrect (key TEXT PRIMARY KEY, value TEXT NOT NULL, added INTEGER NOT NULL) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS shortcuts (key TEXT PRIMARY KEY, value TEXT NOT NULL, added INTEGER NOT NULL) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS choices (key TEXT PRIMARY KEY, value TEXT NOT NULL, added INTEGER NOT NULL) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS learned (key TEXT PRIMARY KEY, uses INTEGER NOT NULL, last INTEGER NOT NULL) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS pairs (prev TEXT NOT NULL, next TEXT NOT NULL, uses INTEGER NOT NULL, last INTEGER NOT NULL, PRIMARY KEY (prev, next)) WITHOUT ROWID;",
        )
        .map_err(|e| e.to_string())?;
        Ok(UserData { db })
    }

    /// Opens an existing user.sqlite for reading only (the Likhon keyboard reads My words and word choices inside other
    /// apps and never writes there). Missing file: an error, nothing is created.
    pub fn open_read_only(path: &Path) -> Result<UserData, String> {
        let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(|e| e.to_string())?;
        db.busy_timeout(std::time::Duration::from_millis(200)).map_err(|e| e.to_string())?;
        Ok(UserData { db })
    }

    /// Checks an entry before it is stored. Returns the cleaned key and value, or the reason it was refused:
    /// "key", "value", "bangla" (My words are Bangla) or "latin" (shortcut codes are Latin letters).
    pub fn check(list: List, key: &str, value: &str) -> Result<(String, String), String> {
        let key = key.trim().to_string();
        let value = value.trim().to_string();
        if key.is_empty() || key.chars().count() > MAX_KEY || key.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err("key".into());
        }
        if value.chars().count() > MAX_VALUE || value.chars().any(|c| c.is_control() && c != '\n') {
            return Err("value".into());
        }
        match list {
            List::Words if !key.chars().all(|c| ('\u{0980}'..='\u{09FF}').contains(&c) || c == '\u{200C}' || c == '\u{200D}') => Err("bangla".into()),
            List::Words => Ok((key, String::new())),
            List::Autocorrect | List::Shortcuts | List::Choices if value.is_empty() => Err("value".into()),
            List::Shortcuts | List::Choices if !key.is_ascii() => Err("latin".into()),
            List::Choices if !value.chars().all(|c| ('\u{0980}'..='\u{09FF}').contains(&c) || c == '\u{200C}' || c == '\u{200D}') => Err("bangla".into()),
            _ => Ok((key, value)),
        }
    }

    pub fn add(&self, list: List, key: &str, value: &str) -> Result<(), String> {
        let (k, v) = Self::check(list, key, value)?;
        if self.count(list) >= MAX_ENTRIES {
            return Err("full".into());
        }
        self.db
            .execute(&format!("INSERT OR REPLACE INTO {} (key, value, added) VALUES (?1, ?2, ?3)", list.table()), rusqlite::params![k, v, now()])
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn remove(&self, list: List, key: &str) -> bool {
        self.db.execute(&format!("DELETE FROM {} WHERE key = ?1", list.table()), [key]).map(|n| n > 0).unwrap_or(false)
    }

    pub fn clear(&self, list: List) {
        let _ = self.db.execute(&format!("DELETE FROM {}", list.table()), []);
    }

    pub fn count(&self, list: List) -> usize {
        self.db.query_row(&format!("SELECT count(*) FROM {}", list.table()), [], |r| r.get::<_, i64>(0)).unwrap_or(0) as usize
    }

    pub fn all(&self, list: List) -> Vec<Entry> {
        let Ok(mut q) = self.db.prepare(&format!("SELECT key, value, added FROM {} ORDER BY key", list.table())) else { return Vec::new() };
        q.query_map([], |r| Ok(Entry { key: r.get(0)?, value: r.get(1)?, added: r.get(2)? })).map(|rows| rows.flatten().collect()).unwrap_or_default()
    }

    pub fn get(&self, list: List, key: &str) -> Option<String> {
        self.db.query_row(&format!("SELECT value FROM {} WHERE key = ?1", list.table()), [key], |r| r.get(0)).optional().ok().flatten()
    }

    /// My words that start with `prefix`, then the learned ones (most used first).
    pub fn words_starting_with(&self, prefix: &str, limit: usize) -> Vec<String> {
        let mut out = prefix_query(&self.db, "SELECT key FROM words WHERE key >= ?1 AND key < ?2 ORDER BY key LIMIT ?3", prefix, limit);
        for w in prefix_query(&self.db, "SELECT key FROM learned WHERE key >= ?1 AND key < ?2 ORDER BY uses DESC, key LIMIT ?3", prefix, limit) {
            if out.len() < limit && !out.contains(&w) {
                out.push(w);
            }
        }
        out
    }

    /// A Bangla word typed in a field confirmed not to be a password field (the agent checks; unsure means it does not
    /// call this). Counted, so words used often come first. Bangla letters only, 2 to 40 of them, not only digits.
    pub fn learn(&self, word: &str) -> bool {
        let n = word.chars().count();
        let bangla = word.chars().all(|c| ('\u{0980}'..='\u{09FF}').contains(&c) || c == '\u{200C}' || c == '\u{200D}');
        if !(2..=40).contains(&n) || !bangla || word.chars().all(|c| ('\u{09E6}'..='\u{09EF}').contains(&c)) || self.learned_count() >= MAX_ENTRIES {
            return false;
        }
        self.db
            .execute("INSERT INTO learned (key, uses, last) VALUES (?1, 1, ?2) ON CONFLICT(key) DO UPDATE SET uses = uses + 1, last = ?2", rusqlite::params![word, now()])
            .is_ok()
    }

    /// Learned words, most used first (the donate review list).
    pub fn learned(&self) -> Vec<String> {
        let Ok(mut q) = self.db.prepare("SELECT key FROM learned ORDER BY uses DESC, key") else { return Vec::new() };
        q.query_map([], |r| r.get(0)).map(|rows| rows.flatten().collect()).unwrap_or_default()
    }

    pub fn learned_count(&self) -> usize {
        self.db.query_row("SELECT count(*) FROM learned", [], |r| r.get::<_, i64>(0)).unwrap_or(0) as usize
    }

    /// Two words typed one after the other (both already learned), for next-word suggestions.
    pub fn learn_pair(&self, prev: &str, next: &str) -> bool {
        let n: i64 = self.db.query_row("SELECT count(*) FROM pairs", [], |r| r.get(0)).unwrap_or(0);
        if n as usize >= MAX_ENTRIES * 4 || prev == next {
            return false;
        }
        self.db
            .execute("INSERT INTO pairs (prev, next, uses, last) VALUES (?1, ?2, 1, ?3) ON CONFLICT(prev, next) DO UPDATE SET uses = uses + 1, last = ?3", rusqlite::params![prev, next, now()])
            .is_ok()
    }

    /// The words most often typed after `prev`.
    pub fn next_words(&self, prev: &str, limit: usize) -> Vec<String> {
        let Ok(mut q) = self.db.prepare("SELECT next FROM pairs WHERE prev = ?1 ORDER BY uses DESC, last DESC LIMIT ?2") else { return Vec::new() };
        q.query_map(rusqlite::params![prev, limit as i64], |r| r.get(0)).map(|rows| rows.flatten().collect()).unwrap_or_default()
    }

    /// Privacy › Forget learned words (and the word pairs learned with them). My words, autocorrect and shortcuts stay.
    pub fn forget_learned(&self) {
        let _ = self.db.execute("DELETE FROM learned", []);
        let _ = self.db.execute("DELETE FROM pairs", []);
    }

    /// A plain text file for the person to keep or share: one entry per line, key and value separated by a tab.
    pub fn export(&self, list: List) -> String {
        let mut out = format!("# Likhon {} · one entry per line{}\n", list.table(), if list == List::Words { "" } else { ", a tab between the two parts" });
        for e in self.all(list) {
            out.push_str(&e.key);
            if list != List::Words {
                out.push('\t');
                out.push_str(&e.value.replace('\n', " "));
            }
            out.push('\n');
        }
        out
    }

    /// Reads a file in the export format (lines starting with # are skipped). Returns (added, skipped). Word choices
    /// also take a space between the two parts, and skip "//" lines, so a list kept by another keyboard reads as is.
    pub fn import(&self, list: List, text: &str) -> (usize, usize) {
        let (mut added, mut skipped) = (0, 0);
        let _ = self.db.execute_batch("BEGIN");
        for line in text.lines().map(|l| l.trim_start_matches('\u{feff}')).filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
            if list == List::Choices && line.starts_with("//") {
                continue;
            }
            let split = line.split_once('\t').or_else(|| if list == List::Choices { line.split_once(' ') } else { None });
            let (k, v) = split.unwrap_or((line, ""));
            if self.add(list, k, v).is_ok() {
                added += 1
            } else {
                skipped += 1
            }
        }
        let _ = self.db.execute_batch("COMMIT");
        (added, skipped)
    }
}

/// May this word be offered in the donation review list? Bangla letters and signs only, 1 to 40 characters, no digits
/// (so no phone, identity or account number) and nothing email-like (Privacy notice: a donation is a word and nothing
/// else). The site checks the same rules again; this keeps such words out of the list the person sees.
pub fn donatable(word: &str) -> bool {
    let n = word.chars().count();
    (1..=40).contains(&n) && word.chars().all(|c| matches!(c, '\u{0980}'..='\u{09FF}' | '\u{200C}' | '\u{200D}') && !matches!(c, '\u{09E6}'..='\u{09EF}'))
}

#[cfg(test)]
mod tests {
    #[test]
    fn donation_list_keeps_only_plain_bangla_words() {
        assert!(super::donatable("ঝালমুড়ি"));
        assert!(!super::donatable("ফোন০১৭১২"));
        assert!(!super::donatable("a@b.com"));
        assert!(!super::donatable("hello"));
        assert!(!super::donatable(""));
        assert!(!super::donatable(&"ক".repeat(41)));
        assert!(super::donatable(&"ক".repeat(40)));
    }

    use super::*;

    fn dict() -> Dictionary {
        Dictionary::open(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/dict/mpl/words.sqlite")).unwrap()
    }

    #[test]
    fn the_built_in_dictionary_opens() {
        let i = dict().info();
        assert_eq!(i.license, "MPL-2.0");
        // The public file holds Avro's 159,380 words; a maintainer's words.full.sqlite (never in git) has more.
        assert!(i.words >= 159_380, "{}", i.words);
        assert!(i.suffixes > 700);
        assert!(i.autocorrect > 2000, "{}", i.autocorrect);
    }

    #[test]
    fn built_in_autocorrect_gives_phonetic_spellings() {
        let a = dict().autocorrect();
        assert!(a.contains(&("account".to_string(), "oZakaunT".to_string())), "{}", a.len());
        assert!(a.iter().all(|(r, p)| r != p && r.chars().any(|c| c.is_ascii_alphabetic())));
    }

    #[test]
    fn learned_words_count_and_forget() {
        let dir = std::env::temp_dir().join(format!("likhon-learn-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let u = UserData::open(&user_db(&dir)).unwrap();
        u.forget_learned();
        assert!(u.learn("আমার") && u.learn("আমার") && u.learn("আমি"));
        assert!(!u.learn("ক"), "too short");
        assert!(!u.learn("abc"), "not Bangla");
        assert!(!u.learn("১২৩৪"), "digits only");
        assert_eq!(u.learned(), vec!["আমার", "আমি"]);
        assert_eq!(u.words_starting_with("আম", 5), vec!["আমার", "আমি"]);
        assert!(u.learn_pair("আমার", "সোনার") && u.learn_pair("আমার", "সোনার") && u.learn_pair("আমার", "দেশ"));
        assert_eq!(u.next_words("আমার", 5), vec!["সোনার", "দেশ"]);
        u.forget_learned();
        assert!(u.next_words("আমার", 5).is_empty());
        assert_eq!(u.learned_count(), 0);
        drop(u);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prefix_search_and_lookup() {
        let d = dict();
        assert!(d.contains("বাংলাদেশ"));
        if d.info().words > 800_000 {
            assert!(d.contains("ভেরিফিকেশনকে"), "an added word, in the larger dictionary only");
        }
        assert!(!d.contains("বাংলাদেশxyz"));
        assert!(!d.contains("অক্টােবর"), "broken spellings were left out");
        let top = d.starting_with("বাংল", 5);
        assert_eq!(top[0], "বাংলা", "the nearest whole word first: {top:?}");
        assert!(top.iter().all(|w| w.chars().count() <= 8), "{top:?}");
        let w = d.starting_with("বাংলা", 50);
        assert!(w.iter().all(|x| x.starts_with("বাংলা")) && w.contains(&"বাংলাদেশ".to_string()), "{w:?}");
        assert_eq!(d.starting_with("বাংলা", 3).len(), 3);
        assert!(d.starting_with("", 5).is_empty());
    }

    #[test]
    fn suffixes_map_latin_endings() {
        assert_eq!(dict().suffix("e").as_deref(), Some("ে"));
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("likhon-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn user_db_renames_the_old_file_once() {
        let dir = std::env::temp_dir().join(format!("likhon-rename-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        {
            let old = UserData::open(&dir.join("user.db")).unwrap();
            old.add(List::Words, "ইচ্ছেমতো", "").unwrap();
        }
        let p = user_db(&dir);
        assert_eq!(p, dir.join("user.sqlite"));
        assert!(!dir.join("user.db").exists());
        let u = UserData::open(&p).unwrap();
        assert_eq!(u.all(List::Words).len(), 1, "the words came along");
        assert_eq!(user_db(&dir), p, "a second call changes nothing");
        drop(u);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lists_add_remove_and_round_trip() {
        let p = temp_path("user.sqlite");
        let u = UserData::open(&p).unwrap();
        u.add(List::Words, "ইচ্ছেমতো", "").unwrap();
        assert!(u.add(List::Words, "hello", "").is_err(), "My words are Bangla");
        u.add(List::Shortcuts, ";ty", "ধন্যবাদ, ভালো থাকবেন।").unwrap();
        assert!(u.add(List::Shortcuts, ";টি", "x").is_err(), "codes are Latin letters");
        u.add(List::Autocorrect, "বিগ্যান", "বিজ্ঞান").unwrap();
        assert_eq!(u.get(List::Autocorrect, "বিগ্যান").as_deref(), Some("বিজ্ঞান"));
        assert_eq!(u.words_starting_with("ইচ্ছে", 5), vec!["ইচ্ছেমতো".to_string()]);
        let file = u.export(List::Shortcuts);
        assert!(u.remove(List::Shortcuts, ";ty"));
        assert_eq!(u.import(List::Shortcuts, &file), (1, 0));
        assert_eq!(u.get(List::Shortcuts, ";ty").as_deref(), Some("ধন্যবাদ, ভালো থাকবেন।"));
        assert_eq!(u.import(List::Autocorrect, "# comment\nএক\tদুই\nbad line without tab\n"), (1, 1));
        // Word choices read a list kept by another keyboard: a "//" header line, then "typed word" per line.
        assert_eq!(u.import(List::Choices, "\u{feff}// Candidate Selection Options\r\nami আমি\r\nmukti মুক্তি\r\nbad bad\r\n"), (2, 1));
        assert_eq!(u.get(List::Choices, "mukti").as_deref(), Some("মুক্তি"));
        drop(u);
        for ext in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", p.display()));
        }
    }

    #[test]
    fn anything_else_is_refused() {
        let tmp = temp_path("not-a-dict.sqlite");
        Connection::open(&tmp).unwrap().execute_batch("CREATE TABLE meta (key TEXT, value TEXT);").unwrap();
        assert!(Dictionary::open(&tmp).is_err());
        let _ = std::fs::remove_file(&tmp);
    }
}
