//! The text tools (v2 §7 Tools, Desktop/CLAUDE.md 8): clean pasted text, Taka in words, numbers and dates with the
//! Bangla calendar, and "how do I type this word". Pure functions, the same as the website's tools page
//! (Web/Laravel/public/js/tools.js), which moves to this code through the WASM build.
//!
//! The Bangla number words and month names are Bangla content; like all Bangla in the project they still need a
//! native speaker's review.

const DIGITS: [char; 10] = ['০', '১', '২', '৩', '৪', '৫', '৬', '৭', '৮', '৯'];

pub fn bn_digits(s: &str) -> String {
    s.chars().map(|c| c.to_digit(10).map(|d| DIGITS[d as usize]).unwrap_or(c)).collect()
}

fn ascii_digits(s: &str) -> String {
    s.chars().map(|c| DIGITS.iter().position(|d| *d == c).map(|i| char::from(b'0' + i as u8)).unwrap_or(c)).collect()
}

/// South Asian grouping: 12,34,567.
fn group(n: u64) -> String {
    let s = n.to_string();
    if s.len() <= 3 {
        return s;
    }
    let (head, last3) = s.split_at(s.len() - 3);
    let mut parts: Vec<String> = Vec::new();
    let mut h = head.to_string();
    while h.len() > 2 {
        parts.insert(0, h[h.len() - 2..].to_string());
        h.truncate(h.len() - 2);
    }
    if !h.is_empty() {
        parts.insert(0, h);
    }
    format!("{},{}", parts.join(","), last3)
}

// ---- Clean pasted text ------------------------------------------------------------------------------

/// Removes invisible characters and stray joiners, joins doubled spaces, removes spaces before the dari and
/// punctuation, and trims each line.
pub fn clean(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().filter(|c| !matches!(c, '\u{200B}' | '\u{FEFF}' | '\u{00AD}' | '\u{2060}')).collect();
    for (i, &c) in chars.iter().enumerate() {
        // A joiner (ZWNJ, ZWJ) next to a space or at an edge does nothing but cause trouble.
        if matches!(c, '\u{200C}' | '\u{200D}') {
            let prev = if i == 0 { None } else { chars.get(i - 1) };
            let next = chars.get(i + 1);
            if prev.is_none_or(|p| p.is_whitespace()) || next.is_none_or(|n| n.is_whitespace()) {
                continue;
            }
        }
        if matches!(c, ' ' | '\t' | '\u{00A0}') {
            if !out.ends_with(' ') {
                out.push(' ');
            }
            continue;
        }
        if matches!(c, '।' | '॥' | ',' | ';' | ':' | '?' | '!') {
            while out.ends_with(' ') {
                out.pop();
            }
        }
        out.push(c);
    }
    out.split('\n').map(str::trim).collect::<Vec<_>>().join("\n").trim().to_string()
}

// ---- Taka in words ------------------------------------------------------------------------------------

const WORDS: [&str; 100] = [
    "", "এক", "দুই", "তিন", "চার", "পাঁচ", "ছয়", "সাত", "আট", "নয়", "দশ", "এগারো", "বারো", "তেরো", "চৌদ্দ", "পনেরো", "ষোলো", "সতেরো", "আঠারো", "উনিশ", "বিশ",
    "একুশ", "বাইশ", "তেইশ", "চব্বিশ", "পঁচিশ", "ছাব্বিশ", "সাতাশ", "আঠাশ", "ঊনত্রিশ", "ত্রিশ", "একত্রিশ", "বত্রিশ", "তেত্রিশ", "চৌত্রিশ", "পঁয়ত্রিশ", "ছত্রিশ", "সাঁইত্রিশ", "আটত্রিশ", "ঊনচল্লিশ", "চল্লিশ",
    "একচল্লিশ", "বিয়াল্লিশ", "তেতাল্লিশ", "চুয়াল্লিশ", "পঁয়তাল্লিশ", "ছেচল্লিশ", "সাতচল্লিশ", "আটচল্লিশ", "ঊনপঞ্চাশ", "পঞ্চাশ", "একান্ন", "বাহান্ন", "তিপ্পান্ন", "চুয়ান্ন", "পঞ্চান্ন", "ছাপ্পান্ন", "সাতান্ন", "আটান্ন", "ঊনষাট", "ষাট",
    "একষট্টি", "বাষট্টি", "তেষট্টি", "চৌষট্টি", "পঁয়ষট্টি", "ছেষট্টি", "সাতষট্টি", "আটষট্টি", "ঊনসত্তর", "সত্তর", "একাত্তর", "বাহাত্তর", "তিয়াত্তর", "চুয়াত্তর", "পঁচাত্তর", "ছিয়াত্তর", "সাতাত্তর", "আটাত্তর", "ঊনআশি", "আশি",
    "একাশি", "বিরাশি", "তিরাশি", "চুরাশি", "পঁচাশি", "ছিয়াশি", "সাতাশি", "অষ্টআশি", "ঊননব্বই", "নব্বই", "একানব্বই", "বিরানব্বই", "তিরানব্বই", "চুরানব্বই", "পঁচানব্বই", "ছিয়ানব্বই", "সাতানব্বই", "আটানব্বই", "নিরানব্বই",
];

/// A whole number in Bangla words, with crore, lakh and thousand (up to 99 crore and more: crores repeat).
pub fn number_words(mut n: u64) -> String {
    if n == 0 {
        return "শূন্য".into();
    }
    let mut parts: Vec<String> = Vec::new();
    let crore = n / 10_000_000;
    if crore > 0 {
        parts.push(format!("{} কোটি", number_words(crore)));
        n %= 10_000_000;
    }
    for (v, name) in [(100_000u64, "লক্ষ"), (1000, "হাজার")] {
        let q = n / v;
        if q > 0 {
            parts.push(format!("{} {name}", WORDS[q as usize]));
            n %= v;
        }
    }
    if n >= 100 {
        parts.push(format!("{}শো", WORDS[(n / 100) as usize]));
        n %= 100;
    }
    if n > 0 {
        parts.push(WORDS[n as usize].to_string());
    }
    parts.join(" ")
}

/// "1,250.50" or "১২৫০" → "এক হাজার দুইশো পঞ্চাশ টাকা পঞ্চাশ পয়সা মাত্র" and the amount written with Bangla digits.
/// None if it is not an amount (or 100 crore or more).
pub fn taka(input: &str) -> Option<(String, String)> {
    let s: String = ascii_digits(input).chars().filter(|c| !matches!(c, ',' | ' ' | '৳' | '\u{00A0}')).collect();
    let (whole, frac) = s.split_once('.').unwrap_or((&s, ""));
    if whole.is_empty() && frac.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) || frac.len() > 2 || !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let t: u64 = if whole.is_empty() { 0 } else { whole.parse().ok()? };
    if t >= 1_000_000_000 {
        return None;
    }
    let p: u64 = if frac.is_empty() { 0 } else { format!("{frac:0<2}").parse().ok()? };
    let mut words = String::new();
    if t > 0 {
        words = format!("{} টাকা", number_words(t));
    }
    if p > 0 {
        if !words.is_empty() {
            words.push(' ');
        }
        words.push_str(&format!("{} পয়সা", number_words(p)));
    }
    if words.is_empty() {
        words = "শূন্য টাকা".into();
    }
    let amount = format!("৳ {}{}", bn_digits(&group(t)), if frac.is_empty() { String::new() } else { format!(".{}", bn_digits(&format!("{p:02}"))) });
    Some((format!("{words} মাত্র"), amount))
}

// ---- Numbers and dates ---------------------------------------------------------------------------------

const MONTHS: [&str; 12] = ["জানুয়ারি", "ফেব্রুয়ারি", "মার্চ", "এপ্রিল", "মে", "জুন", "জুলাই", "আগস্ট", "সেপ্টেম্বর", "অক্টোবর", "নভেম্বর", "ডিসেম্বর"];
const BMONTHS: [&str; 12] = ["বৈশাখ", "জ্যৈষ্ঠ", "আষাঢ়", "শ্রাবণ", "ভাদ্র", "আশ্বিন", "কার্তিক", "অগ্রহায়ণ", "পৌষ", "মাঘ", "ফাল্গুন", "চৈত্র"];
const DAYS: [&str; 7] = ["রবিবার", "সোমবার", "মঙ্গলবার", "বুধবার", "বৃহস্পতিবার", "শুক্রবার", "শনিবার"];

fn leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Days since 1970-01-01 for a Gregorian date (proleptic), or None if the date does not exist.
fn days(y: i64, m: u32, d: u32) -> Option<i64> {
    let len = [31, if leap(y) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1..=12).contains(&m) || d == 0 || d > len[m as usize - 1] {
        return None;
    }
    // Howard Hinnant's days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

/// The Bangladesh calendar date (revised 2019: 14 April starts the year; Boishakh to Ashwin have 31 days, Kartik to
/// Magh and Choitro 30, Falgun 29, or 30 when the Gregorian year it falls in is a leap year): (day, month 0-11, year).
pub fn bangabda(y: i64, m: u32, d: u32) -> Option<(u32, usize, i64)> {
    let today = days(y, m, d)?;
    let start_y = if today >= days(y, 4, 14)? { y } else { y - 1 };
    let mut idx = today - days(start_y, 4, 14)?;
    let lens = [31, 31, 31, 31, 31, 31, 30, 30, 30, 30, if leap(start_y + 1) { 30 } else { 29 }, 30];
    let mut month = 0;
    while idx >= lens[month] {
        idx -= lens[month];
        month += 1;
    }
    Some((idx as u32 + 1, month, start_y - 593))
}

/// The text with Bangla digits; for a date in it (dd/mm/yyyy, dd-mm-yyyy or dd.mm.yyyy), also the date in Bangla,
/// in the Bangla calendar, and the weekday.
pub fn numbers(s: &str) -> String {
    let mut out = bn_digits(s);
    let a = ascii_digits(s);
    let bytes: Vec<char> = a.chars().collect();
    for start in 0..bytes.len() {
        if start > 0 && bytes[start - 1].is_ascii_digit() {
            continue;
        }
        let rest: String = bytes[start..].iter().collect();
        let mut it = rest.splitn(4, |c: char| c == '/' || c == '-' || c == '.');
        let (Some(dd), Some(mm), Some(yy)) = (it.next(), it.next(), it.next()) else { continue };
        let yy: String = yy.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !(1..=2).contains(&dd.len()) || !(1..=2).contains(&mm.len()) || yy.len() != 4 || !dd.chars().all(|c| c.is_ascii_digit()) || !mm.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let (d, m, y): (u32, u32, i64) = (dd.parse().unwrap_or(0), mm.parse().unwrap_or(0), yy.parse().unwrap_or(0));
        if let (Some(n), Some((bd, bm, by))) = (days(y, m, d), bangabda(y, m, d)) {
            let weekday = DAYS[((n + 4).rem_euclid(7)) as usize];
            out.push_str(&format!(
                "\n\n{} {} {} · {} {} {} বঙ্গাব্দ · {weekday}",
                bn_digits(&d.to_string()),
                MONTHS[m as usize - 1],
                bn_digits(&y.to_string()),
                bn_digits(&bd.to_string()),
                BMONTHS[bm],
                bn_digits(&by.to_string())
            ));
            break;
        }
    }
    out
}

// ---- How to type a word (phonetic) ----------------------------------------------------------------------

/// Bangla letters and the phonetic keys for them, longest first where it matters.
const KEYS: &[(&str, &str)] = &[
    ("ক্ষ", "kkh"), ("জ্ঞ", "gg"), ("ঞ্চ", "NC"), ("ঞ্জ", "Nj"), ("ঙ্গ", "Ngg"), ("ঙ্ক", "Ngk"),
    ("ক", "k"), ("খ", "kh"), ("গ", "g"), ("ঘ", "gh"), ("ঙ", "Ng"), ("চ", "c"), ("ছ", "ch"), ("জ", "j"), ("ঝ", "jh"), ("ঞ", "NG"),
    ("ট", "T"), ("ঠ", "Th"), ("ড", "D"), ("ঢ", "Dh"), ("ণ", "N"), ("ত", "t"), ("থ", "th"), ("দ", "d"), ("ধ", "dh"), ("ন", "n"),
    ("প", "p"), ("ফ", "ph"), ("ব", "b"), ("ভ", "bh"), ("ম", "m"), ("য", "z"), ("র", "r"), ("ল", "l"), ("শ", "sh"), ("ষ", "Sh"),
    ("স", "s"), ("হ", "h"), ("ড়", "R"), ("ঢ়", "Rh"), ("য়", "y"), ("ৎ", "t``"), ("ং", "ng"), ("ঃ", ":`"), ("ঁ", "^"),
    ("অ", "o"), ("আ", "a"), ("ই", "i"), ("ঈ", "I"), ("উ", "u"), ("ঊ", "U"), ("ঋ", "rri"), ("এ", "e"), ("ঐ", "OI"), ("ও", "O"), ("ঔ", "OU"),
    ("া", "a"), ("ি", "i"), ("ী", "I"), ("ু", "u"), ("ূ", "U"), ("ৃ", "rri"), ("ে", "e"), ("ৈ", "OI"), ("ো", "O"), ("ৌ", "OU"),
    ("্য", "y"), ("্র", "r"), ("্", ""), ("।", "."), ("০", "0"), ("১", "1"), ("২", "2"), ("৩", "3"), ("৪", "4"), ("৫", "5"), ("৬", "6"), ("৭", "7"), ("৮", "8"), ("৯", "9"),
];

fn is_consonant(c: char) -> bool {
    matches!(c, '\u{0995}'..='\u{09B9}' | '\u{09DC}' | '\u{09DD}' | '\u{09DF}' | '\u{09CE}')
}

/// The keys that type `word` in phonetic, checked by typing them back. A consonant followed by another consonant
/// needs "o" between them when there is no hasanta (ক + ম gives "kom", not "km" which joins them).
pub fn how_to_type(word: &str) -> Option<String> {
    let word = word.trim();
    if word.is_empty() || word.chars().count() > 40 {
        return None;
    }
    let chars: Vec<char> = word.chars().collect();
    let mut keys = String::new();
    let mut i = 0;
    while i < chars.len() {
        let rest: String = chars[i..].iter().collect();
        let (bn, k) = KEYS.iter().find(|(b, _)| rest.starts_with(b))?;
        keys.push_str(k);
        let n = bn.chars().count();
        // An inherent vowel: a consonant followed by another consonant (not joined by a hasanta) is spoken with "o".
        if let (Some(&last), Some(&next)) = (chars.get(i + n - 1), chars.get(i + n)) {
            if is_consonant(last) && is_consonant(next) {
                keys.push('o');
            }
        }
        i += n;
    }
    // Try the plain form first, then without the inherent vowels the rules add on their own.
    for candidate in [keys.clone(), keys.replace("o", "")] {
        if crate::phonetic(&candidate) == word {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taka_matches_the_design_example() {
        // v2 §7 Tools: ১২৫০ → এক হাজার দুইশো পঞ্চাশ টাকা
        let (words, amount) = taka("১২৫০").unwrap();
        assert_eq!(words, "এক হাজার দুইশো পঞ্চাশ টাকা মাত্র");
        assert_eq!(amount, "৳ ১,২৫০");
        assert_eq!(taka("12,34,567.5").unwrap().0, "বারো লক্ষ চৌত্রিশ হাজার পাঁচশো সাতষট্টি টাকা পঞ্চাশ পয়সা মাত্র");
        assert_eq!(taka("0").unwrap().0, "শূন্য টাকা মাত্র");
        assert!(taka("abc").is_none() && taka("1.234").is_none());
    }

    #[test]
    fn crores_group_like_the_website() {
        assert_eq!(number_words(10_00_00_000), "দশ কোটি");
        assert_eq!(group(123456789), "12,34,56,789");
    }

    #[test]
    fn the_bangla_calendar_follows_the_2019_revision() {
        assert_eq!(bangabda(2026, 4, 14), Some((1, 0, 1433)));
        assert_eq!(bangabda(2026, 4, 13), Some((30, 11, 1432)));
        assert_eq!(bangabda(2025, 12, 16), Some((1, 8, 1432)));
        assert_eq!(bangabda(2024, 2, 21), Some((8, 10, 1430)));
    }

    #[test]
    fn numbers_add_the_date_lines() {
        let out = numbers("21/02/1952");
        assert!(out.starts_with("২১/০২/১৯৫২"), "{out}");
        assert!(out.contains("২১ ফেব্রুয়ারি ১৯৫২") && out.contains("বঙ্গাব্দ") && out.contains("বৃহস্পতিবার"), "{out}");
        assert_eq!(numbers("room 12"), "room ১২");
        assert_eq!(numbers("31/02/2024"), "৩১/০২/২০২৪");
    }

    #[test]
    fn cleaning_removes_what_cannot_be_seen() {
        assert_eq!(clean("  আমি\u{200B}  ভাত খাই ।  \n\u{200C}তুমি  "), "আমি ভাত খাই।\nতুমি");
    }

    #[test]
    fn how_to_type_is_checked_by_typing_it_back() {
        for w in ["আমার", "বাংলাদেশ", "কমল", "শিক্ষা"] {
            let k = how_to_type(w).unwrap_or_else(|| panic!("{w}"));
            assert_eq!(crate::phonetic(&k), w, "{w} via {k}");
        }
        assert!(how_to_type("abc").is_none());
    }
}
