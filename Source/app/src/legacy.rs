// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! Legacy output (owner decision 2026-10-22: Bijoy *output* is allowed; a Bijoy keyboard layout is not offered).
//!
//! Typing stays the same; what reaches the app is Bijoy-encoded text for SutonnyMJ-family fonts. Bijoy writes some
//! vowel signs and reph in a different order from Unicode, so a keystroke can change letters further back in the
//! word. The word is therefore kept in Unicode, converted whole after every keystroke, and only the part of the Bijoy
//! text that changed is erased and retyped. Nothing is kept once the word ends.

use likhon_engine::{unicode_to_bijoy, unicode_to_proshika};

#[derive(Default)]
pub struct Legacy {
    /// The word so far, in Unicode (UTF-16 units, the same units the engine counts its backspaces in).
    unicode: Vec<u16>,
    /// What is on screen for it, in Bijoy (UTF-16 units).
    shown: Vec<u16>,
}

impl Legacy {
    pub fn clear(&mut self) {
        self.unicode.clear();
        self.shown.clear();
    }

    /// The engine replaced its last `backspaces` Unicode units with `text`. Returns what to erase and type on screen, in
    /// the legacy encoding named (settings: legacy.encoding).
    pub fn replace(&mut self, backspaces: usize, text: &str, encoding: &str) -> (usize, String) {
        // The engine can reach back past what this word saw (a word typed before the window was ours): erase that much
        // on screen as it is, there is nothing to convert.
        let beyond = backspaces.saturating_sub(self.unicode.len());
        self.unicode.truncate(self.unicode.len().saturating_sub(backspaces));
        self.unicode.extend(text.encode_utf16());
        let word = String::from_utf16_lossy(&self.unicode);
        let next: Vec<u16> = if encoding == "proshika" { unicode_to_proshika(&word) } else { unicode_to_bijoy(&word) }.encode_utf16().collect();
        let same = self.shown.iter().zip(&next).take_while(|(a, b)| a == b).count();
        let erase = self.shown.len() - same + beyond;
        let typed = String::from_utf16_lossy(&next[same..]);
        self.shown = next;
        (erase, typed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retypes_only_what_changed() {
        let mut l = Legacy::default();
        assert_eq!(l.replace(0, "ক", "bijoy"), (0, "K".into()));
        // e-kar after ক goes before it in Bijoy: the whole word is retyped.
        assert_eq!(l.replace(0, "ে", "bijoy"), (1, "†K".into()));
        assert_eq!(l.replace(0, "র", "bijoy"), (0, "i".into()));
        // The engine takes back its last letter.
        assert_eq!(l.replace(1, "ম", "bijoy"), (1, "g".into()));
        l.clear();
        assert_eq!(l.replace(0, "আ", "bijoy"), (0, "Av".into()));
    }

    #[test]
    fn proshika_also_moves_e_kar_in_front() {
        let mut l = Legacy::default();
        assert_eq!(l.replace(0, "ক", "proshika"), (0, "L".into()));
        assert_eq!(l.replace(0, "ে", "proshika"), (1, "\u{AD}L".into()));
    }
}
