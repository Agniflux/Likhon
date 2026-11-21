# MPL-2.0 files in the engine

The files in this folder are covered by the Mozilla Public License 2.0 (`LICENSE` in this folder). MPL is per file:
the rest of the engine is not affected.

## `unicode_to_bijoy.rs`

- **Unicode → Bijoy** is a line-by-line port to Rust of Avro Keyboard's Unicode to Bijoy converter, Copyright
  OmicronLab:
  - https://github.com/mugli/Avro-Keyboard/blob/6766d27fd2c7b41d0ce77fe872f90d2f6348c3b1/Unicode%20to%20ascii%20converter/clsUnicodeToBijoy2000.pas
  - the letter definitions it uses: https://github.com/mugli/Avro-Keyboard/blob/6766d27fd2c7b41d0ce77fe872f90d2f6348c3b1/Keyboard%20and%20Spell%20checker/Units/BanglaChars.pas
- **Bijoy → Unicode** is Likhon's own addition in the same file (so under the same licence): the reverse of that
  glyph map, with the visual order undone (ি ে ৈ moved after the consonant cluster, reph moved before it).

Changes from the source:

- Pascal strings became `Vec<char>`; the 1-based positions, `Pos`, `MidStr` and `WideStuffString` are kept as small
  helpers so the port can be checked against the original step by step.
- `FinalTouch` in the original has six hard-coded pair replacements. In the repository at that commit five of them
  are stored as U+FFFD (the original characters were lost), so they cannot be ported; they are left out. The one
  that survives intact (জু: `Rz` → `Ry`) is kept.
- The curly double quotes that `ReArrangeKars` maps are written as U+201C/U+201D (also lost as U+FFFD in the
  repository); they map to Bijoy's `Ò` and `Ó`.

Owner decision 2026-10-22: legacy *output* (and conversion) is allowed; only a Bijoy keyboard layout is disputed.
Credited on the About page with the other Avro Keyboard files.
