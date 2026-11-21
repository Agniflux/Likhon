//! Emoji by name (Settings › Typing, "Emoji by name"): `:hashi:` or `:smile:` becomes an emoji. Names come from the
//! Unicode CLDR annotations in English and Bangla (emoji.tsv, Unicode License v3, SOURCE.md here).

use std::collections::HashMap;
use std::sync::OnceLock;

const TABLE: &str = include_str!("emoji.tsv");

fn table() -> &'static HashMap<(&'static str, &'static str), &'static str> {
    static T: OnceLock<HashMap<(&str, &str), &str>> = OnceLock::new();
    T.get_or_init(|| {
        TABLE.lines().filter_map(|l| {
            let mut p = l.split('\t');
            Some(((p.next()?, p.next()?), p.next()?))
        }).collect()
    })
}

/// Bangla spellings that phonetic typing cannot tell apart: compared loosely (স শ ষ, ন ণ, long and short ই উ, য য়, জ য).
fn loose(s: &str) -> String {
    s.chars().map(|c| match c {
        'শ' | 'ষ' => 'স',
        'ণ' => 'ন',
        'ী' => 'ি',
        'ূ' => 'ু',
        'ঈ' => 'ই',
        'ঊ' => 'উ',
        'য' => 'জ',
        c => c,
    }).filter(|c| *c != '\u{09BC}').collect()
}

/// Bangla names by their loose form. A name already in its plain form (হাসি) wins over one that only matches
/// loosely (হাশি, chopsticks), so `:hashi:` gives a smile.
fn loose_table() -> &'static HashMap<String, &'static str> {
    static T: OnceLock<HashMap<String, &str>> = OnceLock::new();
    T.get_or_init(|| {
        let mut m = HashMap::new();
        for plain_first in [true, false] {
            for l in TABLE.lines() {
                let mut p = l.split('\t');
                if let (Some("bn"), Some(k), Some(e)) = (p.next(), p.next(), p.next()) {
                    if (loose(k) == k) == plain_first {
                        m.entry(loose(k)).or_insert(e);
                    }
                }
            }
        }
        m
    })
}

/// The emoji for a name typed in Latin letters (what was typed between the colons): an English name as it is, or
/// a Bangla name typed phonetically.
pub fn by_name(keys: &str) -> Option<&'static str> {
    if keys.is_empty() || keys.len() > 30 || !keys.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
        return None;
    }
    // Bangla first (this is Bangla typing: "hashi" is a smile, not English CLDR's chopsticks), then English.
    let t = table();
    let bn = crate::phonetic(keys);
    loose_table().get(&loose(&bn)).copied().or_else(|| t.get(&("en", keys.to_ascii_lowercase().as_str())).copied())
}

#[cfg(test)]
mod tests {
    use super::by_name;

    #[test]
    fn names_in_english_and_bangla() {
        assert_eq!(by_name("smile"), Some("😀"));
        assert_eq!(by_name("heart"), Some("❤️"));
        assert_eq!(by_name("hasi"), Some("😀"));
        assert_eq!(by_name("hashi"), Some("😀"), "শ and স compared loosely");
        assert_eq!(by_name("dhonyobad"), Some("🙏"));
        assert_eq!(by_name("nosuchname"), None);
        assert_eq!(by_name("a b"), None);
    }
}
