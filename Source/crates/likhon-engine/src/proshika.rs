// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! Unicode Bangla to Proshika, the legacy encoding of Proshika Computer Systems' Lipi fonts (1995). Likhon's own code;
//! the map is `proshika.tsv`, read from the fonts' glyphs without real Proshika documents to check against, so its
//! "unsure" rows (and the order of reph, assumed from Bijoy) need checking with real text.
//!
//! A word is built syllable by syllable: the vowel signs ি ে ৈ go in front of the consonant group, then the group (a
//! conjunct drawn as one glyph where the font has it, else a phala form, else consonant + hasanta + consonant), then
//! reph, then the other vowel signs and ঁ ং ঃ. Text that is not Bangla passes through.

use std::collections::HashMap;
use std::sync::OnceLock;

const HASANTA: char = '\u{9CD}';
const NUKTA: char = '\u{9BC}';
const RA: char = '\u{9B0}';
const ZWJ: char = '\u{200D}';
const ZWNJ: char = '\u{200C}';
const PRE_BASE: [char; 3] = ['\u{9BF}', '\u{9C7}', '\u{9C8}']; // ি ে ৈ
const POST_BASE: [char; 6] = ['\u{9BE}', '\u{9C0}', '\u{9C1}', '\u{9C2}', '\u{9C3}', '\u{9D7}']; // া ী ু ূ ৃ ৗ
const SIGNS: [char; 3] = ['\u{981}', '\u{982}', '\u{983}']; // ঁ ং ঃ
/// Consonants that have a phala form in the font (written "-X" in the table).
const PHALA: [char; 6] = ['\u{9AF}', '\u{9B0}', '\u{9AC}', '\u{9B2}', '\u{9A8}', '\u{9AE}']; // য র ব ল ন ম

struct Table {
    map: HashMap<String, String>,
    /// The longest key, in chars, for the conjunct search.
    longest: usize,
}

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        let mut map = HashMap::new();
        for line in include_str!("proshika.tsv").lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let mut cols = line.split('\t');
            let (Some(key), Some(bytes)) = (cols.next(), cols.next()) else { continue };
            let text: String = bytes.split(' ').filter_map(|b| u8::from_str_radix(b, 16).ok()).map(cp1252).collect();
            map.insert(normalize(key).into_iter().collect::<String>(), text);
        }
        let longest = map.keys().map(|k| k.chars().count()).max().unwrap_or(1);
        Table { map, longest }
    })
}

/// A byte as the Windows-1252 character the fonts map to the same glyph (what the app types).
fn cp1252(b: u8) -> char {
    const HIGH: [u16; 32] = [
        0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x8D, 0x017D, 0x8F,
        0x90, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x9D, 0x017E, 0x0178,
    ];
    if (0x80..0xA0).contains(&b) { char::from_u32(HIGH[(b - 0x80) as usize] as u32).unwrap_or('?') } else { b as char }
}

/// One form for the same text: ড় ঢ় য় as single letters, ো and ৌ split into their two parts.
fn normalize(text: &str) -> Vec<char> {
    let mut out: Vec<char> = Vec::with_capacity(text.len());
    for c in text.chars() {
        match c {
            NUKTA => match out.last() {
                Some('\u{9A1}') => *out.last_mut().unwrap() = '\u{9DC}',
                Some('\u{9A2}') => *out.last_mut().unwrap() = '\u{9DD}',
                Some('\u{9AF}') => *out.last_mut().unwrap() = '\u{9DF}',
                _ => {}
            },
            '\u{9CB}' => out.extend(['\u{9C7}', '\u{9BE}']),
            '\u{9CC}' => out.extend(['\u{9C7}', '\u{9D7}']),
            _ => out.push(c),
        }
    }
    out
}

fn is_consonant(c: char) -> bool {
    matches!(c, '\u{995}'..='\u{9A8}' | '\u{9AA}'..='\u{9B0}' | '\u{9B2}' | '\u{9B6}'..='\u{9B9}' | '\u{9DC}' | '\u{9DD}' | '\u{9DF}')
}

/// Converts Unicode Bangla text to Proshika-encoded text.
pub fn unicode_to_proshika(text: &str) -> String {
    let t = table();
    let s = normalize(text);
    let get = |k: &str| t.map.get(k).cloned();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        // Reph: র + hasanta before a consonant (with ZWJ it is র + ya-phala instead, handled as a group).
        let reph = s[i] == RA && s.get(i + 1) == Some(&HASANTA) && s.get(i + 2).is_some_and(|&c| is_consonant(c));
        if reph {
            i += 2;
        }
        if !is_consonant(s[i]) {
            match get(&s[i].to_string()) {
                Some(g) => out.push_str(&g),
                None if matches!(s[i], ZWJ | ZWNJ | NUKTA) => {}
                None => out.push(s[i]),
            }
            i += 1;
            continue;
        }
        // The consonant group: consonants joined by hasanta (a ZWJ after it keeps the join).
        let mut group = vec![s[i]];
        let mut trailing_hasanta = false;
        i += 1;
        while s.get(i) == Some(&HASANTA) {
            let mut j = i + 1;
            if s.get(j) == Some(&ZWJ) {
                j += 1;
            }
            match s.get(j) {
                Some(&c) if is_consonant(c) => {
                    group.push(c);
                    i = j + 1;
                }
                _ => {
                    trailing_hasanta = true;
                    i = if s.get(i + 1) == Some(&ZWNJ) { i + 2 } else { i + 1 };
                    break;
                }
            }
        }
        // The vowel signs and ঁ ং ঃ after it.
        let (mut pre, mut post, mut signs) = (Vec::new(), Vec::new(), Vec::new());
        while let Some(&c) = s.get(i) {
            if PRE_BASE.contains(&c) {
                pre.push(c);
            } else if POST_BASE.contains(&c) {
                post.push(c);
            } else if SIGNS.contains(&c) {
                signs.push(c);
            } else if c != NUKTA {
                break;
            }
            i += 1;
        }
        for c in &pre {
            out.push_str(&get(&c.to_string()).unwrap_or_else(|| c.to_string()));
        }
        // A consonant and its ু or ূ drawn as one glyph (গু শু হু রু রূ).
        let mut post = &post[..];
        let joined = (group.len() == 1 && !trailing_hasanta).then(|| post.first().and_then(|k| get(&format!("{}{k}", group[0])))).flatten();
        match joined {
            Some(g) => {
                out.push_str(&g);
                post = &post[1..];
            }
            None => out.push_str(&render_group(t, &group)),
        }
        if trailing_hasanta {
            out.push_str(&get(&HASANTA.to_string()).unwrap_or_default());
        }
        if reph {
            out.push_str(&get("reph").unwrap_or_default());
        }
        for c in post.iter().chain(&signs) {
            out.push_str(&get(&c.to_string()).unwrap_or_else(|| c.to_string()));
        }
    }
    out
}

/// A consonant group: the longest conjunct the font draws as one glyph, else a phala form, else hasanta + consonant.
fn render_group(t: &Table, group: &[char]) -> String {
    let key = |cs: &[char]| cs.iter().map(char::to_string).collect::<Vec<_>>().join(&HASANTA.to_string());
    let mut out = String::new();
    let mut pos = 0;
    while pos < group.len() {
        let most = (group.len() - pos).min(t.longest.div_ceil(2));
        if let Some((k, g)) = (2..=most).rev().find_map(|k| t.map.get(&key(&group[pos..pos + k])).map(|g| (k, g))) {
            if pos > 0 {
                out.push_str(t.map.get(&HASANTA.to_string()).map(String::as_str).unwrap_or_default());
            }
            out.push_str(g);
            pos += k;
            continue;
        }
        let c = group[pos];
        if pos > 0 {
            match PHALA.contains(&c).then(|| t.map.get(&format!("-{c}"))).flatten() {
                Some(g) => {
                    out.push_str(g);
                    pos += 1;
                    continue;
                }
                None => out.push_str(t.map.get(&HASANTA.to_string()).map(String::as_str).unwrap_or_default()),
            }
        }
        match t.map.get(&c.to_string()) {
            Some(g) => out.push_str(g),
            None => out.push(c),
        }
        pos += 1;
    }
    out
}

// ---- Proshika to Unicode ----------------------------------------------------------------------------------------

/// Stands for reph while the text is still in Proshika's visual order.
const REPH_MARK: char = '\u{E000}';

/// Glyphs only the reverse direction needs: the other widths of vowel signs and phala forms, and half forms on their
/// own (proshika.tsv names them in its notes and comments).
const REVERSE_EXTRA: [(u8, &str); 19] = [
    (0xA5, "\u{9C1}"), (0xA8, "\u{9C2}"), (0xAB, "\u{9C3}"), (0xAE, "\u{9C7}"), (0xB0, "\u{9C8}"),
    (0xCB, "্র"), (0xCC, "্র"), (0xD0, "্র"), (0xC5, "্ব"), (0xC6, "্ব"), (0xC1, "্ন"),
    (0xD8, "স্"), (0xD9, "স্"), (0xD6, "ষ্"), (0xD7, "ষ্"), (0xB9, "্ত"), (0xBB, "্ত্র"), (0xBC, "্দ"), (0xA6, "\u{9C1}"),
];

struct Reverse {
    map: HashMap<String, String>,
    longest: usize,
}

fn reverse() -> &'static Reverse {
    static R: OnceLock<Reverse> = OnceLock::new();
    R.get_or_init(|| {
        // The table's own rows first. Two rows with the same glyphs keep the shorter text, so a plain letter never
        // turns into a joined form; the order of the rows does not matter.
        let mut map: HashMap<String, String> = HashMap::new();
        for (key, glyphs) in &table().map {
            let unicode = if key == "reph" {
                REPH_MARK.to_string()
            } else if let Some(c) = key.strip_prefix('-') {
                format!("{HASANTA}{c}")
            } else {
                key.clone()
            };
            let keep_old = map.get(glyphs).is_some_and(|old| (old.chars().count(), old.as_str()) <= (unicode.chars().count(), unicode.as_str()));
            if !keep_old {
                map.insert(glyphs.clone(), unicode);
            }
        }
        // Then the glyphs only this direction needs, where the table has no row of its own.
        for (b, u) in REVERSE_EXTRA {
            map.entry(cp1252(b).to_string()).or_insert_with(|| u.to_string());
        }
        let longest = map.keys().map(|k| k.chars().count()).max().unwrap_or(1);
        Reverse { map, longest }
    })
}

/// Converts Proshika-encoded text (the Lipi fonts) back to Unicode Bangla: glyphs to letters, longest match first, then
/// ি ে ৈ moved back after their consonant group, reph back in front of it as র + hasanta, and ে + া → ো, ে + ৗ → ৌ.
/// Characters that are not in the table pass through. Like every legacy encoding, Latin letters in the text are read as
/// Proshika glyphs too.
pub fn proshika_to_unicode(text: &str) -> String {
    let r = reverse();
    let src: Vec<char> = text.chars().collect();
    // 1. Glyphs to letters, still in Proshika's visual order.
    let mut s: Vec<char> = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        let most = (src.len() - i).min(r.longest);
        match (1..=most).rev().find_map(|k| r.map.get(&src[i..i + k].iter().collect::<String>()).map(|u| (k, u))) {
            Some((k, u)) => {
                s.extend(u.chars());
                i += k;
            }
            None => {
                s.push(src[i]);
                i += 1;
            }
        }
    }
    // A first half (স্) before a second half (্ত) leaves a doubled hasanta.
    let mut t: Vec<char> = Vec::with_capacity(s.len());
    for c in s {
        if !(c == HASANTA && t.last() == Some(&HASANTA)) {
            t.push(c);
        }
    }
    let s = t;

    // The consonant group starting at `i`: consonant (hasanta consonant)*. Returns where it ends.
    let group_end = |s: &[char], mut i: usize| -> usize {
        if i >= s.len() || !is_consonant(s[i]) {
            return i;
        }
        i += 1;
        while i + 1 < s.len() && s[i] == HASANTA && is_consonant(s[i + 1]) {
            i += 2;
        }
        i
    };

    // 2. Reph follows its group in Proshika: put র + hasanta in front of the group.
    let mut out: Vec<char> = Vec::with_capacity(s.len() + 4);
    for &c in &s {
        if c != REPH_MARK {
            out.push(c);
            continue;
        }
        // Back over exactly one group: its last consonant, then each (consonant, hasanta) before it.
        let mut start = out.len();
        if start > 0 && is_consonant(out[start - 1]) {
            start -= 1;
            while start >= 2 && out[start - 1] == HASANTA && is_consonant(out[start - 2]) {
                start -= 2;
            }
        }
        out.splice(start..start, [RA, HASANTA]);
    }

    // 3. Pre-base vowel signs come before their group in Proshika: move each after it, joining ো and ৌ.
    let s = out;
    let mut out: Vec<char> = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if PRE_BASE.contains(&c) && i + 1 < s.len() && is_consonant(s[i + 1]) {
            let end = group_end(&s, i + 1);
            out.extend(&s[i + 1..end]);
            match (c, s.get(end)) {
                ('\u{9C7}', Some('\u{9BE}')) => {
                    out.push('\u{9CB}');
                    i = end + 1;
                }
                ('\u{9C7}', Some('\u{9D7}')) => {
                    out.push('\u{9CC}');
                    i = end + 1;
                }
                _ => {
                    out.push(c);
                    i = end;
                }
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    // 4. ড় ঢ় য় in Unicode's normal form (letter + nukta), as typed text and the other converters give them.
    let mut text = String::with_capacity(out.len() * 3);
    for c in out {
        match c {
            '\u{9DC}' => text.push_str("\u{9A1}\u{9BC}"),
            '\u{9DD}' => text.push_str("\u{9A2}\u{9BC}"),
            '\u{9DF}' => text.push_str("\u{9AF}\u{9BC}"),
            _ => text.push(c),
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Proshika text for a list of bytes, as it is typed.
    fn bytes(b: &[u8]) -> String {
        b.iter().map(|&b| cp1252(b)).collect()
    }

    #[test]
    fn letters_and_vowel_signs() {
        assert_eq!(unicode_to_proshika("আমি"), bytes(&[0x42, 0xA2, 0x6A]));
        assert_eq!(unicode_to_proshika("বাংলা"), bytes(&[0x68, 0xA1, 0x77, 0x6D, 0xA1]));
        assert_eq!(unicode_to_proshika("কেন"), bytes(&[0xAD, 0x4C, 0x65]));
        assert_eq!(unicode_to_proshika("কোন"), bytes(&[0xAD, 0x4C, 0xA1, 0x65]));
        assert_eq!(unicode_to_proshika("বৌ"), bytes(&[0xAD, 0x68, 0xB1]));
        assert_eq!(unicode_to_proshika("১৯৭১।"), bytes(&[0x31, 0x39, 0x37, 0x31, 0x7A]));
    }

    #[test]
    fn nukta_forms_are_one_letter() {
        assert_eq!(unicode_to_proshika("বড়"), bytes(&[0x68, 0x73]));
        assert_eq!(unicode_to_proshika("বড\u{9BC}"), bytes(&[0x68, 0x73]));
    }

    #[test]
    fn conjuncts_phala_reph_and_hasanta() {
        assert_eq!(unicode_to_proshika("ক্ষমা"), bytes(&[0x72, 0x6A, 0xA1]));
        assert_eq!(unicode_to_proshika("বিদ্যা"), bytes(&[0xA2, 0x68, 0x63, 0xC9, 0xA1]));
        assert_eq!(unicode_to_proshika("কর্ম"), bytes(&[0x4C, 0x6A, 0xD1]));
        assert_eq!(unicode_to_proshika("সন্ধ্যা"), bytes(&[0x70, 0xE5, 0xC9, 0xA1]));
        assert_eq!(unicode_to_proshika("স্কুল"), bytes(&[0xFA, 0xA4, 0x6D]));
        assert_eq!(unicode_to_proshika("স্ত্রী"), bytes(&[0xF2, 0xA3]));
        // Half forms: ন and a second-half ত্র; a first-half স and থ; ক and a second-half ত.
        assert_eq!(unicode_to_proshika("মন্ত্রী"), bytes(&[0x6A, 0x65, 0xBB, 0xA3]));
        assert_eq!(unicode_to_proshika("স্থান"), bytes(&[0xD8, 0x62, 0xA1, 0x65]));
        assert_eq!(unicode_to_proshika("রক্ত"), bytes(&[0x6C, 0x4C, 0xB9]));
        // No one-glyph ঙ্ঘ: ঙ, hasanta, ঘ.
        assert_eq!(unicode_to_proshika("ঙ্ঘ"), bytes(&[0x50, 0xDA, 0x4F]));
        assert_eq!(unicode_to_proshika("প্রথম"), bytes(&[0x66, 0xCA, 0x62, 0x6A]));
        assert_eq!(unicode_to_proshika("ক্\u{200C}"), bytes(&[0x4C, 0xDA]));
    }

    #[test]
    fn consonant_and_u_as_one_glyph() {
        assert_eq!(unicode_to_proshika("গুরু"), bytes(&[0x85, 0x6C, 0xA6]));
        assert_eq!(unicode_to_proshika("শুভ"), bytes(&[0xF6, 0x69]));
    }

    #[test]
    fn other_text_passes_through() {
        assert_eq!(unicode_to_proshika("Likhon 2026"), "Likhon 2026");
        assert_eq!(unicode_to_proshika(""), "");
    }

    #[test]
    fn every_row_of_the_table_is_read() {
        let t = table();
        assert!(t.map.len() > 120, "{} rows", t.map.len());
        assert!(t.map.values().all(|v| !v.is_empty()));
    }

    #[test]
    fn proshika_back_to_unicode_round_trips() {
        let words = "আমি বাংলা ভাষা কর্ম ক্ষমা বিদ্যা সন্ধ্যা স্কুল স্ত্রী মন্ত্রী প্রথম গুরু শুভ রূপ কৌশল বৌদ্ধ পৃথিবী             চন্দ্র ঐক্য বিজ্ঞান সংগ্রাম শিক্ষা আর্থিক স্বাধীনতা স্থান রক্ত কেন কোন বড় বাড়ি দেশ নদী আকাশ পাখি গান             কার্যকর সূর্য ধর্ম অর্থনীতি মুক্তিযুদ্ধ ১৯৭১ সাল। শব্দ গ্রন্থ পত্রিকা বাঁশি বৈশাখ চৈত্র যাত্রা";
        for w in words.split_whitespace() {
            assert_eq!(proshika_to_unicode(&unicode_to_proshika(w)), w, "{w}");
        }
        // The sample in Settings › Tools › Converter and the website's converter.
        assert_eq!(unicode_to_proshika("আমি বাংলায় গান গাই।"), "B¢j h¡wm¡u N¡e N¡Cz");
        // A whole sentence, with spaces and punctuation kept.
        let line = "আমি বাংলায় গান গাই।";
        assert_eq!(proshika_to_unicode(&unicode_to_proshika(line)), line);
    }

    #[test]
    fn other_widths_and_half_forms_read_back() {
        // A5 is the other ু, CB the other ra-phala, D8 a first-half স on its own.
        assert_eq!(proshika_to_unicode(&bytes(&[0x4C, 0xA5])), "কু");
        assert_eq!(proshika_to_unicode(&bytes(&[0x66, 0xCB])), "প্র");
        assert_eq!(proshika_to_unicode(&bytes(&[0xD8, 0x62])), "স্থ");
    }
}
