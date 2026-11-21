//! The app's text, compiled from the shared text library by build.rs (root CLAUDE.md 5c).

include!(concat!(env!("OUT_DIR"), "/text.rs"));

/// Interface language of the native windows.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Bn,
}

/// One phrase in a language, with {name} placeholders filled. Unknown IDs show the ID, so a
/// missing phrase is visible in development instead of blank.
pub fn t(id: &str, lang: Lang, vars: &[(&str, &str)]) -> String {
    let mut s = PHRASES
        .iter()
        .find(|(k, _, _)| *k == id)
        .map(|(_, en, bn)| if lang == Lang::Bn { *bn } else { *en })
        .unwrap_or(id)
        .to_string();
    for (k, v) in vars {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

/// NUL-terminated UTF-16 for Win32.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrases_come_from_the_text_library() {
        assert_eq!(t("app.tray.exit", Lang::En, &[]), "Exit");
        assert_eq!(t("app.tray.exit", Lang::Bn, &[]), "বন্ধ করুন");
        assert_eq!(t("app.tray.tipEnglish", Lang::En, &[("key", "F12")]), "Likhon: English (F12 for Bangla)");
        assert_eq!(t("no.such.id", Lang::En, &[]), "no.such.id");
    }
}
