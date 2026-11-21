// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! The suggestion rows for a typed word and which one starts selected, shared by likhon.exe's suggestion window
//! (app/src/suggest.rs) and the Likhon keyboard's (tsf/src/suggest.rs), so both modes offer the same words.

use crate::Dictionary;

/// One row: the word, and its tag ("auto" for autocorrect, "mine" for My words).
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub word: String,
    pub tag: Tag,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tag {
    Plain,
    Auto,
    Mine,
    Dict,
}

/// Spellings people mix up when typing phonetically: each group's letters are tried in place of each other.
const CONFUSABLE: &[&[char]] = &[&['ি', 'ী'], &['ু', 'ূ'], &['ই', 'ঈ'], &['উ', 'ঊ'], &['স', 'শ', 'ষ'], &['ন', 'ণ'], &['জ', 'য'], &['র', '\u{09DC}'], &['ত', 'ৎ'], &['ং', 'ঙ']];

/// Other spellings of `word` with confusable letters swapped, at most `limit`, the word itself first.
pub fn variants(word: &str, limit: usize) -> Vec<String> {
    let mut out = vec![word.to_string()];
    let mut i = 0;
    while i < out.len() && out.len() < limit {
        let base: Vec<char> = out[i].chars().collect();
        for (pos, c) in base.iter().enumerate() {
            if let Some(group) = CONFUSABLE.iter().find(|g| g.contains(c)) {
                for alt in group.iter().filter(|a| *a != c) {
                    let mut v = base.clone();
                    v[pos] = *alt;
                    let s: String = v.into_iter().collect();
                    if !out.contains(&s) && out.len() < limit {
                        out.push(s);
                    }
                }
            }
        }
        i += 1;
    }
    out
}

/// The rows for a typed word: autocorrect, plain phonetic, then the person's words and dictionary words.
pub fn candidates(text: &str, auto: Option<&str>, mine: &[String], dict: Option<&Dictionary>, limit: usize) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let push = |w: &str, tag: Tag, out: &mut Vec<Candidate>| {
        if !w.is_empty() && out.len() < limit && !out.iter().any(|c| c.word == w) {
            out.push(Candidate { word: w.to_string(), tag });
        }
    };
    if let Some(a) = auto {
        push(a, Tag::Auto, &mut out);
    }
    push(text, Tag::Plain, &mut out);
    for w in mine {
        push(w, Tag::Mine, &mut out);
    }
    if let Some(d) = dict {
        // Whole words first (the typed spelling or a confusable one), then longer words that start the same way.
        for v in variants(text, 24).iter().skip(1) {
            if d.contains(v) {
                push(v, Tag::Dict, &mut out);
            }
        }
        for w in d.starting_with(text, limit * 2) {
            push(&w, Tag::Dict, &mut out);
        }
    }
    out
}

/// Which row starts selected. "dict": a whole-word correction (autocorrect, or a dictionary spelling of the same
/// length) when the typed spelling is not a word itself; "plain": what was typed.
pub fn preselect(items: &[Candidate], text: &str, dict: Option<&Dictionary>, prefer_dict: bool) -> usize {
    if let Some(i) = items.iter().position(|c| c.tag == Tag::Auto) {
        return i;
    }
    if prefer_dict && !dict.is_some_and(|d| d.contains(text)) {
        let n = text.chars().count();
        if let Some(i) = items.iter().position(|c| c.tag == Tag::Dict && c.word.chars().count() == n) {
            return i;
        }
    }
    items.iter().position(|c| c.tag == Tag::Plain).unwrap_or(0)
}

/// Puts an imported word choice into the rows (at the top, tagged as the person's, if it is not there yet) and
/// returns its row.
pub fn choose(items: &mut Vec<Candidate>, choice: String, limit: usize) -> usize {
    if let Some(i) = items.iter().position(|c| c.word == choice) {
        return i;
    }
    items.insert(0, Candidate { word: choice, tag: Tag::Mine });
    items.truncate(limit.max(1));
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict() -> Dictionary {
        Dictionary::open(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/dict/mpl/words.sqlite")).unwrap()
    }

    #[test]
    fn variants_swap_confusable_letters() {
        let v = variants("শিখি", 10);
        assert_eq!(v[0], "শিখি");
        assert!(v.contains(&"শিখী".to_string()) && v.contains(&"সিখি".to_string()));
    }

    #[test]
    fn order_is_autocorrect_plain_mine_dictionary() {
        let d = dict();
        let c = candidates("বাংলা", Some("বাংলাদেশ"), &["বাংলায়".to_string()], Some(&d), 5);
        assert_eq!(c[0], Candidate { word: "বাংলাদেশ".into(), tag: Tag::Auto });
        assert_eq!(c[1], Candidate { word: "বাংলা".into(), tag: Tag::Plain });
        assert_eq!(c[2], Candidate { word: "বাংলায়".into(), tag: Tag::Mine });
        // Then dictionary words: a confusable spelling that is a word (বাঙলা), then longer words.
        assert!(c.len() == 5 && c[3..].iter().all(|x| x.tag == Tag::Dict), "{c:?}");
    }

    #[test]
    fn a_dictionary_spelling_is_preselected_only_for_a_misspelling() {
        let d = dict();
        // শিক্ষক is a word: keep what was typed.
        let ok = candidates("শিক্ষক", None, &[], Some(&d), 5);
        assert_eq!(ok[preselect(&ok, "শিক্ষক", Some(&d), true)].word, "শিক্ষক");
        // বাড়ী (a confusable spelling) is offered as বাড়ি when that is the dictionary's word.
        let typo = candidates("বাড়ী", None, &[], Some(&d), 5);
        let pick = &typo[preselect(&typo, "বাড়ী", Some(&d), true)].word;
        assert!(d.contains(pick), "{pick}");
        assert_eq!(typo[preselect(&typo, "বাড়ী", Some(&d), false)].word, "বাড়ী");
    }
}
