//! Likhon typing engine (root CLAUDE.md, section 5a): pure logic, no I/O.
//!
//! Phonetic typing on top of `okkhor` (MIT), and fixed layouts from JSON layout files (`fixed`).
//! Autocorrect, ranking, converters and the Taka and number tools come later, tests first against the
//! owner's corpus in `tests/corpus/`.

pub mod emoji;
pub mod fixed;
pub mod tools;
/// Files under the Mozilla Public License 2.0, ported from Avro Keyboard (src/mpl/SOURCE.md). MPL is per file.
pub mod mpl {
    pub mod unicode_to_bijoy;
}
pub use mpl::unicode_to_bijoy::{bijoy_to_unicode, unicode_to_bijoy};
/// Proshika output, Likhon's own map read from the Lipi fonts (src/proshika.tsv; unsure rows need real text).
pub mod proshika;
pub use proshika::{proshika_to_unicode, unicode_to_proshika};
/// Files under the MIT licence from the Khipro team (src/mit/SOURCE.md).
pub mod mit {
    pub mod khipro;
}

use okkhor::editor::Editor;
use okkhor::parser::Parser;

/// Engine version, reported by `likhon.exe --version`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the caller must do after one keystroke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The engine did not take the key: deliver it unchanged.
    PassThrough,
    /// The engine took the key: swallow it, erase `backspaces` characters (UTF-16 units) before
    /// the caret, then type `output`. Both may be empty and the key is still swallowed.
    Replace { backspaces: usize, output: String },
}

/// Typing options from Settings, Typing page (Reference/design-archive/S02Typing.dc.html).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// "Bangla digits": 1 gives ১. Off: digits stay as typed.
    pub bangla_digits: bool,
    /// "Full stop as dari": `.` gives ।, and `..` gives a full stop. Off: `.` stays a full stop.
    pub dari: bool,
    /// "Colon as visarga": `:` gives ঃ (the Avro way). Off: `:` stays a colon.
    pub visarga: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { bangla_digits: false, dari: true, visarga: true }
    }
}

/// Phonetic typing, one keystroke at a time.
pub struct Phonetic {
    editor: Editor,
    options: Options,
    /// The last key typed was a `.` that became ।, so a second `.` turns it into a full stop.
    after_dari: bool,
}

impl Default for Phonetic {
    fn default() -> Self {
        Self::new()
    }
}

impl Phonetic {
    pub fn new() -> Self {
        Self::with_options(Options::default())
    }

    pub fn with_options(options: Options) -> Self {
        Self { editor: Editor::new_phonetic(), options, after_dari: false }
    }

    pub fn set_options(&mut self, options: Options) {
        self.options = options;
        self.reset();
    }

    /// A typed character. Characters that cannot be part of a phonetic word (space,
    /// punctuation the rules do not use, non-ASCII) end the word and pass through.
    pub fn key(&mut self, ch: char) -> Action {
        let after_dari = std::mem::replace(&mut self.after_dari, false);
        if ch == '.' {
            self.editor.put_non_char();
            if !self.options.dari {
                return Action::PassThrough;
            }
            if after_dari {
                // ".." : the । just typed becomes a full stop.
                return Action::Replace { backspaces: 1, output: ".".into() };
            }
            self.after_dari = true;
            return Action::Replace { backspaces: 0, output: "।".into() };
        }
        if ch == ':' && !self.options.visarga {
            self.editor.put_non_char();
            return Action::PassThrough;
        }
        if ch.is_ascii_digit() && !self.options.bangla_digits {
            self.editor.put_non_char();
            return Action::PassThrough;
        }
        let e = self.editor.put_char(ch);
        if e.pass_through {
            Action::PassThrough
        } else {
            Action::Replace { backspaces: e.backspaces, output: e.output.to_string() }
        }
    }

    /// Anything that may move the caret or change the text outside the engine: Backspace,
    /// Enter, arrows, a mouse click, another window. The current word ends; nothing on screen
    /// changes.
    pub fn reset(&mut self) {
        self.editor.put_non_char();
        self.after_dari = false;
    }
}

/// Convert a whole phonetic string at once (website tools, tests).
pub fn phonetic(input: &str) -> String {
    Parser::new_phonetic().convert(input)
}

/// What typing `keys` one by one with these options leaves on screen (Settings, Learn practice).
pub fn type_keys(keys: &str, options: Options) -> String {
    let mut engine = Phonetic::with_options(options);
    let mut screen: Vec<u16> = Vec::new();
    for ch in keys.chars() {
        match engine.key(ch) {
            Action::PassThrough => {
                let mut b = [0u16; 2];
                screen.extend_from_slice(ch.encode_utf16(&mut b));
            }
            Action::Replace { backspaces, output } => {
                screen.truncate(screen.len().saturating_sub(backspaces));
                screen.extend(output.encode_utf16());
            }
        }
    }
    String::from_utf16_lossy(&screen)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays keystrokes into a text buffer the way the hook does on screen.
    fn type_into(keys: &str) -> String {
        let mut engine = Phonetic::new();
        let mut screen: Vec<char> = Vec::new();
        for ch in keys.chars() {
            match engine.key(ch) {
                Action::PassThrough => screen.push(ch),
                Action::Replace { backspaces, output } => {
                    for _ in 0..backspaces {
                        screen.pop();
                    }
                    screen.extend(output.chars());
                }
            }
        }
        screen.into_iter().collect()
    }

    #[test]
    fn spike_a_target_ami() {
        // Root CLAUDE.md M2: "ami" gives আমি.
        assert_eq!(type_into("ami"), "আমি");
    }

    #[test]
    fn whole_sentences_match_the_converter() {
        for s in ["amar sOnar bangla", "ami banglay gan gai", "kemon achho", "shikkha", "bangladesh"] {
            let typed = type_into(s);
            let whole: String = s.split(' ').map(phonetic).collect::<Vec<_>>().join(" ");
            assert_eq!(typed, whole, "{s}");
        }
    }

    #[test]
    fn examples_from_the_design() {
        // Reference/design-archive/WebFeatures.dc.html, WebHome.dc.html
        assert_eq!(phonetic("amar"), "আমার");
        assert_eq!(phonetic("bangladesh"), "বাংলাদেশ");
        assert_eq!(phonetic("shikkha"), "শিক্ষা");
    }

    #[test]
    fn reset_starts_a_new_word_without_touching_the_screen() {
        let mut e = Phonetic::new();
        e.key('k');
        e.reset();
        // After a reset, "a" is a new word: আ, not া after ক.
        assert_eq!(e.key('a'), Action::Replace { backspaces: 0, output: "আ".into() });
    }

    fn type_with(options: Options, keys: &str) -> String {
        type_keys(keys, options)
    }

    #[test]
    fn digits_follow_the_bangla_digits_option() {
        assert_eq!(type_with(Options { bangla_digits: false, dari: true, visarga: true }, "ami 12"), "আমি 12");
        assert_eq!(type_with(Options { bangla_digits: true, dari: true, visarga: true }, "ami 12"), "আমি ১২");
    }

    #[test]
    fn dot_gives_dari_and_two_dots_give_a_full_stop() {
        let on = Options { bangla_digits: false, dari: true, visarga: true };
        assert_eq!(type_with(on, "ami."), "আমি।");
        assert_eq!(type_with(on, "ami.."), "আমি.");
        assert_eq!(type_with(on, "ami. tumi."), "আমি। তুমি।");
        assert_eq!(type_with(Options { bangla_digits: false, dari: false, visarga: true }, "ami."), "আমি.");
    }

    #[test]
    fn colon_follows_the_visarga_option() {
        let on = Options { bangla_digits: false, dari: true, visarga: true };
        let off = Options { visarga: false, ..on };
        assert_eq!(type_with(on, "du:kho"), "দুঃখ");
        assert_eq!(type_with(off, "ami:"), "আমি:");
        assert_eq!(type_with(off, "du:kho").contains('ঃ'), false);
    }

    #[test]
    fn space_passes_through() {
        let mut e = Phonetic::new();
        e.key('a');
        assert_eq!(e.key(' '), Action::PassThrough);
    }
}

