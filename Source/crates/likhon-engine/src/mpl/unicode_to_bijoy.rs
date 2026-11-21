// =============================================================================
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// =============================================================================
//
//! Unicode Bangla to Bijoy (the legacy ASCII encoding of the SutonnyMJ family of fonts), ported line by line from
//! Avro Keyboard's `clsUnicodeToBijoy2000.pas`, and Bijoy back to Unicode (Likhon's own reverse of the same map) (see SOURCE.md in this folder for the exact commit and the changes).
//! Owner decision 2026-10-22: legacy *output* is allowed; only a Bijoy keyboard layout is disputed.
//!
//! The steps run in Avro's order: denormalise nukta forms, move the pre-base vowel signs (ি ে ৈ) and reph in front,
//! map vowel signs and vowels, r-fola / z-fola / hasanta, full conjunct forms, first-half and second-half forms, the
//! consonants, then drop what is left of hasanta, ZWJ and ZWNJ. Positions are 1-based to keep the port checkable
//! against the Pascal source.

/// Converts Unicode Bangla text to Bijoy-encoded text. Text that is not Bangla passes through.
pub fn unicode_to_bijoy(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut s: Vec<char> = text.chars().collect();
    denormalize(&mut s);
    s = rearrange_kars(&s);
    s = rearrange_reph(&s);
    replace_kars_vowels(&mut s);
    convert_rfola_zfola_hasanta(&mut s);
    replace_full_forms(&mut s);
    first_half_forms(&mut s);
    second_half_forms(&mut s);
    consonants(&mut s);
    final_touch(&mut s);
    s.into_iter().collect()
}

// ---- Bangla (Avro's BanglaChars.pas; Pascal names are case-insensitive, so b_s and b_S are the same) ----------
const B_0: char = '\u{9E6}';
const B_A: char = '\u{985}';
const B_AA: char = '\u{986}';
const B_AAKAR: char = '\u{9BE}';
const B_I: char = '\u{987}';
const B_II: char = '\u{988}';
const B_IIKAR: char = '\u{9C0}';
const B_IKAR: char = '\u{9BF}';
const B_U: char = '\u{989}';
const B_UKAR: char = '\u{9C1}';
const B_UU: char = '\u{98A}';
const B_UUKAR: char = '\u{9C2}';
const B_RRI: char = '\u{98B}';
const B_RRIKAR: char = '\u{9C3}';
const B_E: char = '\u{98F}';
const B_EKAR: char = '\u{9C7}';
const B_O: char = '\u{993}';
const B_OI: char = '\u{990}';
const B_OIKAR: char = '\u{9C8}';
const B_OKAR: char = '\u{9CB}';
const B_OU: char = '\u{994}';
const B_OUKAR: char = '\u{9CC}';
const B_ANUSHAR: char = '\u{982}';
const B_B: char = '\u{9AC}';
const B_BH: char = '\u{9AD}';
const B_BISHARGA: char = '\u{983}';
const B_C: char = '\u{99A}';
const B_CH: char = '\u{99B}';
const B_CHANDRA: char = '\u{981}';
const B_D: char = '\u{9A6}';
const B_DD: char = '\u{9A1}';
const B_DDH: char = '\u{9A2}';
const B_DH: char = '\u{9A7}';
const B_G: char = '\u{997}';
const B_GH: char = '\u{998}';
const B_H: char = '\u{9B9}';
const B_J: char = '\u{99C}';
const B_JH: char = '\u{99D}';
const B_K: char = '\u{995}';
const B_KH: char = '\u{996}';
const B_L: char = '\u{9B2}';
const B_M: char = '\u{9AE}';
const B_N: char = '\u{9A8}';
const B_NGA: char = '\u{999}';
const B_NN: char = '\u{9A3}';
const B_NYA: char = '\u{99E}';
const B_P: char = '\u{9AA}';
const B_PH: char = '\u{9AB}';
const B_R: char = '\u{9B0}';
const B_RR: char = '\u{9DC}';
const B_RRH: char = '\u{9DD}';
const B_S: char = '\u{9B8}';
const B_SH: char = '\u{9B6}';
const B_SS: char = '\u{9B7}';
const B_T: char = '\u{9A4}';
const B_TH: char = '\u{9A5}';
const B_TT: char = '\u{99F}';
const B_TTH: char = '\u{9A0}';
const B_Y: char = '\u{9DF}';
const B_Z: char = '\u{9AF}';
const ASSAM_RA: char = '\u{9F0}';
const ASSAM_VA: char = '\u{9F1}';
const B_KHANDATTA: char = '\u{9CE}';
const B_NUKTA: char = '\u{9BC}';
const B_LENGTH_MARK: char = '\u{9D7}';
const B_DARI: char = '\u{964}';
const B_DOUBLE_DARI: char = '\u{965}';
const B_HASANTA: char = '\u{9CD}';
const B_TAKA: char = '\u{9F3}';
const ZWJ: char = '\u{200D}';
const ZWNJ: char = '\u{200C}';

fn is_pure_consonant(c: char) -> bool {
    matches!(
        c,
        B_B | B_BH | B_C | B_CH | B_D | B_DD | B_DDH | B_DH | B_G | B_GH | B_H | B_J | B_JH | B_K | B_KH | B_L | B_M | B_N | B_NGA | B_NN | B_NYA
            | B_P | B_PH | B_R | B_RR | B_RRH | B_S | B_SH | B_SS | B_T | B_TH | B_TT | B_TTH | B_Z | B_Y | B_KHANDATTA | ASSAM_RA | ASSAM_VA
    )
}

// ---- Bijoy 2000 font map ----------------------------------------------------------------------------------------
const A_A: char = '\u{41}';
const A_AAKAR: char = '\u{76}';
const A_I: char = '\u{42}';
const A_IKAR: char = '\u{77}';
const A_II: char = '\u{43}';
const A_IIKAR: char = '\u{78}';
const A_U: char = '\u{44}';
const A_UKAR2: char = '\u{79}';
const A_UKAR1: char = '\u{7A}';
const A_UKAR3: char = '\u{2013}';
const A_UKAR4: char = '\u{201C}';
const A_UU: char = '\u{45}';
const A_UUKAR2: char = '\u{7E}';
const A_UUKAR1: char = '\u{201A}';
const A_UUKAR3: char = '\u{192}';
const A_RRI: char = '\u{46}';
const A_RRIKAR1: char = '\u{201E}';
const A_RRIKAR2: char = '\u{2026}';
const A_E: char = '\u{47}';
const A_EKAR1: char = '\u{2020}';
const A_EKAR2: char = '\u{2021}';
const A_OI: char = '\u{48}';
const A_OIKAR1: char = '\u{2C6}';
const A_OIKAR2: char = '\u{2030}';
const A_O: char = '\u{49}';
const A_OU: char = '\u{4A}';
const A_OUKAR: char = '\u{160}';
const A_TAKA: char = '\u{24}';
const A_DARI: char = '\u{7C}';
const A_DOUBLE_DANDA: char = '\u{5C}';
const A_HASANTA: char = '\u{26}';
const A_START_DOUBLE_QUOTE: char = '\u{D2}';
const A_END_DOUBLE_QUOTE: char = '\u{D3}';
const A_K: char = '\u{4B}';
const A_KH: char = '\u{4C}';
const A_G: char = '\u{4D}';
const A_GH: char = '\u{4E}';
const A_NGA: char = '\u{4F}';
const A_C: char = '\u{50}';
const A_CH: char = '\u{51}';
const A_J: char = '\u{52}';
const A_JH: char = '\u{53}';
const A_NYA: char = '\u{54}';
const A_TT: char = '\u{55}';
const A_TTH: char = '\u{56}';
const A_DD: char = '\u{57}';
const A_DDH: char = '\u{58}';
const A_NN: char = '\u{59}';
const A_T: char = '\u{5A}';
const A_TH: char = '\u{5F}';
const A_D: char = '\u{60}';
const A_DH: char = '\u{61}';
const A_N: char = '\u{62}';
const A_P: char = '\u{63}';
const A_PH: char = '\u{64}';
const A_B: char = '\u{65}';
const A_BH: char = '\u{66}';
const A_M: char = '\u{67}';
const A_Z: char = '\u{68}';
const A_R: char = '\u{69}';
const A_L: char = '\u{6A}';
const A_SH: char = '\u{6B}';
const A_SS: char = '\u{6C}';
const A_S: char = '\u{6D}';
const A_H: char = '\u{6E}';
const A_RR: char = '\u{6F}';
const A_RRH: char = '\u{70}';
const A_Y: char = '\u{71}';
const A_KHANDATA: char = '\u{72}';
const A_ANUSHAR: char = '\u{73}';
const A_BISHARGA: char = '\u{74}';
const A_CHANDRA: char = '\u{75}';
// Full forms
const A_K_K: char = '\u{B0}';
const A_K_TT: char = '\u{B1}';
const A_K_SS_M: char = '\u{B2}';
const A_K_T: char = '\u{B3}';
const A_K_M: char = '\u{B4}';
const A_K_R: char = '\u{B5}';
const A_K_SS: char = '\u{B6}';
const A_K_S: char = '\u{B7}';
const A_G_UKAR: char = '\u{B8}';
const A_G_G: char = '\u{B9}';
const A_G_D: char = '\u{BA}';
const A_G_DH: char = '\u{BB}';
const A_NGA_K: char = '\u{BC}';
const A_NGA_G: char = '\u{BD}';
const A_J_J: char = '\u{BE}';
const A_J_JH: char = '\u{C0}';
const A_J_NYA: char = '\u{C1}';
const A_NYA_C: char = '\u{C2}';
const A_NYA_CH: char = '\u{C3}';
const A_NYA_J: char = '\u{C4}';
const A_NYA_JH: char = '\u{C5}';
const A_TT_TT: char = '\u{C6}';
const A_DD_DD: char = '\u{C7}';
const A_NN_TT: char = '\u{C8}';
const A_NN_TTH: char = '\u{C9}';
const A_NN_DD: char = '\u{CA}';
const A_T_T: char = '\u{CB}';
const A_T_TH: char = '\u{CC}';
const A_T_M: char = '\u{CD}';
const A_T_R: char = '\u{CE}';
const A_D_D: char = '\u{CF}';
const A_D_DH: char = '\u{D7}';
const A_D_B: char = '\u{D8}';
const A_D_M: char = '\u{D9}';
const A_N_TTH: char = '\u{DA}';
const A_N_DD: char = '\u{DB}';
const A_N_DH: char = '\u{DC}';
const A_N_S: char = '\u{DD}';
const A_P_TT: char = '\u{DE}';
const A_P_T: char = '\u{DF}';
const A_P_P: char = '\u{E0}';
const A_P_S: char = '\u{E1}';
const A_B_J: char = '\u{E2}';
const A_B_D: char = '\u{E3}';
const A_B_DH: char = '\u{E4}';
const A_BH_R: char = '\u{E5}';
const A_M_N: char = '\u{E6}';
const A_M_PH: char = '\u{E7}';
const A_L_K: char = '\u{E9}';
const A_L_G: char = '\u{EA}';
const A_L_TT: char = '\u{EB}';
const A_L_DD: char = '\u{EC}';
const A_L_P: char = '\u{ED}';
const A_L_PH: char = '\u{EE}';
const A_SH_UKAR: char = '\u{EF}';
const A_SH_C: char = '\u{F0}';
const A_SH_CH: char = '\u{F1}';
const A_SS_NN: char = '\u{F2}';
const A_SS_TT: char = '\u{F3}';
const A_SS_TTH: char = '\u{F4}';
const A_SS_PH: char = '\u{F5}';
const A_S_KH: char = '\u{F6}';
const A_S_TT: char = '\u{F7}';
const A_S_N: char = '\u{F8}';
const A_S_PH: char = '\u{F9}';
const A_H_UKAR: char = '\u{FB}';
const A_H_RRIKAR: char = '\u{FC}';
const A_H_N: char = '\u{FD}';
const A_H_M: char = '\u{FE}';
const A_RR_G: char = '\u{FF}';
// First half forms
const A_REPH: char = '\u{A9}';
const A_M_1H: char = '\u{A4}';
const A_SS_1H: char = '\u{AE}';
const A_S_1H_1: char = '\u{AF}';
const A_N_1H_1: char = '\u{161}';
const A_D_1H_1: char = '\u{2DC}';
const A_C_1H: char = '\u{201D}';
const A_NGA_1H: char = '\u{2022}';
const A_N_1H_2: char = '\u{203A}';
const A_D_1H_2: char = '\u{2122}';
// Second half forms
const A_B_2H_1: char = '\u{5E}';
const A_B_2H_2: char = '\u{A1}';
const A_BH_2H: char = '\u{A2}';
const A_BH_R_2H: char = '\u{A3}';
const A_M_2H_1: char = '\u{A5}';
const A_B_2H_3: char = '\u{A6}';
const A_M_2H_2: char = '\u{A7}';
const A_ZFOLA: char = '\u{A8}';
const A_RFOLA_1: char = '\u{AA}';
const A_RFOLA_2: char = '\u{AB}';
const A_L_2H_1: char = '\u{AC}';
const A_T_R_2H: char = '\u{BF}';
const A_RFOLA_3: char = '\u{D6}';
const A_NN_2H_1: char = '\u{E8}';
const A_K_R_2H: char = '\u{152}';
const A_NN_2H_2: char = '\u{153}';
const A_B_2H_4: char = '\u{178}';
const A_T_2H: char = '\u{2014}';
const A_T_UKAR_2H: char = '\u{2018}';
const A_TH_2H: char = '\u{2019}';
const A_K_2H: char = '\u{2039}';
const A_L_2H_3: char = '\u{2212}';

// ---- Pascal string helpers (1-based) ------------------------------------------------------------------------------

/// ReplaceStr: every occurrence, left to right.
fn replace(s: &mut Vec<char>, from: &[char], to: &[char]) {
    if from.is_empty() || s.len() < from.len() {
        return;
    }
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    *s = out;
}

/// Pos: the 1-based position of the first occurrence, or 0.
fn pos(s: &[char], pat: &[char]) -> isize {
    s.windows(pat.len()).position(|w| w == pat).map_or(0, |p| p as isize + 1)
}

/// MidStr (Copy): `len` characters from 1-based `start`; a start before 1 shortens the count, as Delphi does.
fn mid(s: &[char], start: isize, len: isize) -> Vec<char> {
    let (mut start, mut len) = (start, len);
    if start < 1 {
        len += start - 1;
        start = 1;
    }
    if len <= 0 || start as usize > s.len() {
        return Vec::new();
    }
    let a = start as usize - 1;
    let b = (a + len as usize).min(s.len());
    s[a..b].to_vec()
}

fn mid_is(s: &[char], start: isize, len: isize, want: &[char]) -> bool {
    mid(s, start, len) == want
}

/// The single character at 1-based `i`, if any.
fn at(s: &[char], i: isize) -> Option<char> {
    (i >= 1).then(|| s.get(i as usize - 1).copied()).flatten()
}

fn set(s: &mut [char], i: isize, c: char) {
    s[i as usize - 1] = c;
}

/// WideStuffString: replaces `len` characters at 1-based `start` with `sub`.
fn stuff(s: &mut Vec<char>, start: isize, len: isize, sub: &[char]) {
    let a = (start - 1).max(0) as usize;
    let b = ((start - 1 + len).max(0) as usize).min(s.len());
    s.splice(a..b, sub.iter().copied());
}

fn base_line_right(c: Option<char>) -> bool {
    matches!(
        c,
        Some(B_KH | B_G | B_GH | B_NN | B_TH | B_D | B_DH | B_N | B_P | B_B | B_M | B_Z | B_R | B_L | B_SH | B_SS | B_S | B_H | B_Y)
    )
}

// ---- The steps ----------------------------------------------------------------------------------------------------

fn denormalize(s: &mut Vec<char>) {
    replace(s, &[B_Z, B_NUKTA], &[B_Y]);
    replace(s, &[B_DD, B_NUKTA], &[B_RR]);
    replace(s, &[B_DDH, B_NUKTA], &[B_RRH]);
}

fn rearrange_kars(src: &[char]) -> Vec<char> {
    let mut t = src.to_vec();
    // Break O-kar and OU-kar.
    replace(&mut t, &[B_OKAR], &[B_EKAR, B_AAKAR]);
    replace(&mut t, &[B_OUKAR], &[B_EKAR, B_LENGTH_MARK]);

    // Bring i-kar, e-kar and oi-kar to the beginning of the consonant or conjunct, walking backwards.
    let movable = |c: char| matches!(c, B_EKAR | B_IKAR | B_OIKAR);
    let joiner = |c: char| matches!(c, B_HASANTA | ZWJ | ZWNJ);
    let mut out: Vec<char> = Vec::new();
    let mut kar: Option<char> = None;
    let mut i = t.len() as isize;
    while i >= 1 {
        let c = t[i as usize - 1];
        if movable(c) {
            kar = Some(c);
        } else if let Some(k) = kar {
            if i - 1 < 1 {
                // Beginning of the text: the pending kar goes first.
                out.splice(0..0, [k, c]);
                kar = None;
            } else if !is_pure_consonant(c) && !joiner(c) {
                // Start of a consonant cluster reached: the kar goes after this character.
                out.splice(0..0, [c, k]);
                kar = None;
            } else {
                if joiner(c) {
                    out.insert(0, c);
                }
                if is_pure_consonant(c) {
                    if joiner(t[i as usize - 2]) {
                        out.insert(0, c);
                    } else {
                        out.splice(0..0, [k, c]);
                        kar = None;
                    }
                }
            }
        } else {
            out.insert(0, c);
        }
        i -= 1;
    }
    replace(&mut out, &['\u{201C}'], &[A_START_DOUBLE_QUOTE]);
    replace(&mut out, &['\u{201D}'], &[A_END_DOUBLE_QUOTE]);
    out
}

fn rearrange_reph(t: &[char]) -> Vec<char> {
    if t.len() < 3 {
        return t.to_vec();
    }
    let joiner = |c: char| matches!(c, B_HASANTA | ZWJ | ZWNJ);
    let movable = |i: usize| i + 2 < t.len() && t[i] == B_R && t[i + 1] == B_HASANTA && t[i + 2] != ZWJ && t[i + 2] != ZWNJ;
    let mut out: Vec<char> = Vec::new();
    let mut pending = false;
    let mut i = 0usize; // 0-based here: the Pascal loop only looks at I, I+1 and I+2.
    while i < t.len() {
        let c = t[i];
        if !movable(i) {
            if !pending {
                out.push(c);
            } else if !is_pure_consonant(c) && !joiner(c) {
                out.extend([A_REPH, c]);
                pending = false;
            } else if i + 1 >= t.len() {
                out.extend([c, A_REPH]);
                pending = false;
            } else {
                if joiner(c) {
                    out.push(c);
                }
                if is_pure_consonant(c) {
                    if joiner(t[i + 1]) {
                        out.push(c);
                    } else {
                        out.extend([c, A_REPH]);
                        pending = false;
                    }
                }
            }
        } else {
            pending = true;
            i += 1;
        }
        i += 1;
    }
    out
}

fn replace_kars_vowels(s: &mut Vec<char>) {
    let at_word_start = |s: &[char], i: isize| i == 1 || matches!(at(s, i - 1), Some(' ' | '\r' | '\n' | '\t'));
    loop {
        let i = pos(s, &[B_EKAR]);
        if i <= 0 {
            break;
        }
        let c = if at_word_start(s, i) { A_EKAR1 } else { A_EKAR2 };
        set(s, i, c);
    }
    loop {
        let i = pos(s, &[B_OIKAR]);
        if i <= 0 {
            break;
        }
        let c = if at_word_start(s, i) { A_OIKAR1 } else { A_OIKAR2 };
        set(s, i, c);
    }

    // U-kar
    replace(s, &[B_G, B_UKAR], &[A_G_UKAR]);
    replace(s, &[B_SH, B_UKAR], &[A_SH_UKAR]);
    replace(s, &[B_H, B_UKAR], &[A_H_UKAR]);
    replace(s, &[B_HASANTA, B_T, B_UKAR], &[B_HASANTA, A_T_UKAR_2H]);
    let r_cluster = |s: &[char], i: isize| {
        [B_SH, B_D, B_G, B_T, B_J, B_TH, B_DH, B_P, B_B, B_BH, B_M, B_S].iter().any(|&c| mid_is(s, i - 3, 3, &[c, B_HASANTA, B_R]))
            || mid_is(s, i - 5, 5, &[B_N, B_HASANTA, B_D, B_HASANTA, B_R])
            || mid_is(s, i - 5, 5, &[B_M, B_HASANTA, B_P, B_HASANTA, B_R])
            || mid_is(s, i - 5, 5, &[B_SS, B_HASANTA, B_P, B_HASANTA, B_R])
            || mid_is(s, i - 5, 5, &[B_S, B_HASANTA, B_P, B_HASANTA, B_R])
    };
    let l_cluster = |s: &[char], i: isize| {
        [B_G, B_P, B_B, B_SH, B_S].iter().any(|&c| mid_is(s, i - 3, 3, &[c, B_HASANTA, B_L]))
            || mid_is(s, i - 5, 5, &[B_S, B_HASANTA, B_P, B_HASANTA, B_L])
    };
    loop {
        let i = pos(s, &[B_UKAR]);
        if i <= 0 {
            break;
        }
        let c = if i - 1 >= 1 {
            let prev = at(s, i - 1);
            if base_line_right(prev) {
                let mut c = if prev == Some(B_R) {
                    if r_cluster(s, i) || at(s, i - 2) != Some(B_HASANTA) { A_UKAR4 } else { A_UKAR2 }
                } else if prev == Some(B_L) {
                    if l_cluster(s, i) { A_UKAR4 } else { A_UKAR2 }
                } else {
                    A_UKAR2
                };
                if mid_is(s, i - 3, 3, &[B_SS, B_HASANTA, B_NN]) {
                    c = A_UKAR1;
                }
                c
            } else if matches!(prev, Some(B_RR | B_RRH)) {
                A_UKAR3
            } else {
                A_UKAR1
            }
        } else {
            A_UKAR1
        };
        set(s, i, c);
    }

    // UU-kar
    loop {
        let i = pos(s, &[B_UUKAR]);
        if i <= 0 {
            break;
        }
        let c = if i - 1 >= 1 && base_line_right(at(s, i - 1)) {
            let prev = at(s, i - 1);
            if prev == Some(B_R) {
                if r_cluster(s, i) || at(s, i - 2) != Some(B_HASANTA) { A_UUKAR3 } else { A_UUKAR2 }
            } else if prev == Some(B_L) {
                if l_cluster(s, i) { A_UUKAR3 } else { A_UUKAR2 }
            } else {
                A_UUKAR2
            }
        } else {
            A_UUKAR1
        };
        set(s, i, c);
    }

    // RRI-kar
    replace(s, &[B_H, B_RRIKAR], &[A_H_RRIKAR]);
    loop {
        let i = pos(s, &[B_RRIKAR]);
        if i <= 0 {
            break;
        }
        let c = if i - 1 >= 1 && base_line_right(at(s, i - 1)) { A_RRIKAR1 } else { A_RRIKAR2 };
        set(s, i, c);
    }

    // The rest of the kars, then the vowels.
    replace(s, &[B_AAKAR], &[A_AAKAR]);
    replace(s, &[B_IKAR], &[A_IKAR]);
    replace(s, &[B_IIKAR], &[A_IIKAR]);
    replace(s, &[B_LENGTH_MARK], &[A_OUKAR]);
    replace(s, &[B_A], &[A_A]);
    replace(s, &[B_AA], &[A_A, A_AAKAR]);
    replace(s, &[B_I], &[A_I]);
    replace(s, &[B_II], &[A_II]);
    replace(s, &[B_U], &[A_U]);
    replace(s, &[B_UU], &[A_UU]);
    replace(s, &[B_RRI], &[A_RRI]);
    replace(s, &[B_E], &[A_E]);
    replace(s, &[B_OI], &[A_OI]);
    replace(s, &[B_O], &[A_O]);
    replace(s, &[B_OU], &[A_OU]);
}

fn convert_rfola_zfola_hasanta(s: &mut Vec<char>) {
    replace(s, &[B_HASANTA, B_Z], &[A_ZFOLA]);
    replace(s, &[B_HASANTA, ZWNJ], &[A_HASANTA]);
    loop {
        let i = pos(s, &[B_HASANTA, B_R]);
        if i <= 0 {
            break;
        }
        let prev = at(s, i - 1);
        if matches!(prev, Some(B_P | B_G)) {
            stuff(s, i, 2, &[A_RFOLA_3]);
        } else if prev == Some(B_BH) {
            if at(s, i - 2) == Some(B_HASANTA) {
                stuff(s, i - 1, 3, &[A_BH_R_2H]);
            } else {
                stuff(s, i - 1, 3, &[A_BH_R]);
            }
        } else if prev == Some(B_K) {
            if at(s, i - 2) == Some(B_HASANTA) {
                stuff(s, i - 1, 3, &[A_K_R_2H]);
            } else {
                stuff(s, i - 1, 3, &[A_K_R]);
            }
        } else if prev == Some(B_T) {
            if at(s, i - 2) == Some(B_HASANTA) {
                if matches!(at(s, i - 3), Some(B_K | B_T)) {
                    stuff(s, i, 2, &[A_RFOLA_2]);
                } else {
                    stuff(s, i - 1, 3, &[A_T_R_2H]);
                }
            } else {
                stuff(s, i - 1, 3, &[A_T_R]);
            }
        } else if prev == Some(B_PH) {
            stuff(s, i, 2, &[A_RFOLA_2]);
        } else {
            stuff(s, i, 2, &[A_RFOLA_1]);
        }
    }
}

fn replace_full_forms(s: &mut Vec<char>) {
    // Digits
    for d in 0..10u32 {
        replace(s, &[char::from_u32(B_0 as u32 + d).unwrap()], &[char::from_u32('0' as u32 + d).unwrap()]);
    }
    // Symbols
    replace(s, &[B_TAKA], &[A_TAKA]);
    replace(s, &[B_DARI], &[A_DARI]);
    replace(s, &[B_DOUBLE_DARI], &[A_DOUBLE_DANDA]);
    // Other full forms
    replace(s, &[B_KHANDATTA], &[A_KHANDATA]);
    replace(s, &[B_ANUSHAR], &[A_ANUSHAR]);
    replace(s, &[B_BISHARGA], &[A_BISHARGA]);
    replace(s, &[B_CHANDRA], &[A_CHANDRA]);
    let h = B_HASANTA;
    let pairs: &[(&[char], char)] = &[
        (&[B_K, h, B_K], A_K_K),
        (&[B_K, h, B_TT], A_K_TT),
        (&[B_K, h, B_SS, h, B_M], A_K_SS_M),
        (&[B_K, h, B_T], A_K_T),
        (&[B_K, h, B_M], A_K_M),
        (&[B_K, h, B_SS], A_K_SS),
        (&[B_K, h, B_S], A_K_S),
        (&[B_G, h, B_G], A_G_G),
        (&[B_G, h, B_D], A_G_D),
        (&[B_G, h, B_DH], A_G_DH),
        (&[B_NGA, h, B_K], A_NGA_K),
        (&[B_NGA, h, B_G], A_NGA_G),
        (&[B_J, h, B_J], A_J_J),
        (&[B_J, h, B_JH], A_J_JH),
        (&[B_J, h, B_NYA], A_J_NYA),
        (&[B_NYA, h, B_C], A_NYA_C),
        (&[B_NYA, h, B_CH], A_NYA_CH),
        (&[B_NYA, h, B_J], A_NYA_J),
        (&[B_NYA, h, B_JH], A_NYA_JH),
        (&[B_TT, h, B_TT], A_TT_TT),
        (&[B_DD, h, B_DD], A_DD_DD),
        (&[B_NN, h, B_TT], A_NN_TT),
        (&[B_NN, h, B_TTH], A_NN_TTH),
        (&[B_NN, h, B_DD], A_NN_DD),
        (&[B_T, h, B_T], A_T_T),
        (&[B_T, h, B_TH], A_T_TH),
        (&[B_T, h, B_M], A_T_M),
        (&[B_D, h, B_D], A_D_D),
        (&[B_D, h, B_DH], A_D_DH),
        (&[B_D, h, B_B], A_D_B),
        (&[B_D, h, B_M], A_D_M),
        (&[B_N, h, B_TTH], A_N_TTH),
        (&[B_N, h, B_DD], A_N_DD),
        (&[B_N, h, B_DH], A_N_DH),
        (&[B_N, h, B_S], A_N_S),
        (&[B_P, h, B_TT], A_P_TT),
        (&[B_P, h, B_T], A_P_T),
        (&[B_P, h, B_P], A_P_P),
        (&[B_P, h, B_S], A_P_S),
        (&[B_B, h, B_J], A_B_J),
        (&[B_B, h, B_D], A_B_D),
        (&[B_B, h, B_DH], A_B_DH),
        (&[B_M, h, B_N], A_M_N),
        (&[B_M, h, B_PH], A_M_PH),
        (&[B_L, h, B_K], A_L_K),
        (&[B_L, h, B_G], A_L_G),
        (&[B_L, h, B_TT], A_L_TT),
        (&[B_L, h, B_DD], A_L_DD),
        (&[B_L, h, B_P], A_L_P),
        (&[B_L, h, B_PH], A_L_PH),
        (&[B_SH, h, B_C], A_SH_C),
        (&[B_SH, h, B_CH], A_SH_CH),
        (&[B_SS, h, B_NN], A_SS_NN),
        (&[B_SS, h, B_TT], A_SS_TT),
        (&[B_SS, h, B_TTH], A_SS_TTH),
        (&[B_SS, h, B_PH], A_SS_PH),
        (&[B_S, h, B_KH], A_S_KH),
        (&[B_S, h, B_TT], A_S_TT),
        (&[B_S, h, B_N], A_S_N),
        (&[B_S, h, B_PH], A_S_PH),
        (&[B_H, h, B_N], A_H_N),
        (&[B_H, h, B_M], A_H_M),
        (&[B_RR, h, B_G], A_RR_G),
    ];
    for (from, to) in pairs {
        replace(s, from, &[*to]);
    }
}

fn first_half_forms(s: &mut Vec<char>) {
    let h = B_HASANTA;
    replace(s, &[B_M, h], &[A_M_1H, h]);
    replace(s, &[B_SS, h], &[A_SS_1H, h]);
    replace(s, &[B_C, h], &[A_C_1H, h]);
    replace(s, &[B_NGA, h], &[A_NGA_1H, h]);
    replace(s, &[B_S, h], &[A_S_1H_1, h]);
    loop {
        let i = pos(s, &[B_D, h]);
        if i <= 0 {
            break;
        }
        let c = if at(s, i + 2) == Some(B_G) { A_D_1H_1 } else { A_D_1H_2 };
        set(s, i, c);
    }
    loop {
        let i = pos(s, &[B_N, h]);
        if i <= 0 {
            break;
        }
        let next = at(s, i + 2);
        let c = if matches!(next, Some(B_T | B_TH | B_L | B_B | A_T_R_2H | A_T_UKAR_2H)) {
            A_N_1H_1
        } else if matches!(next, Some(B_M | B_N)) {
            A_N
        } else {
            A_N_1H_2
        };
        set(s, i, c);
    }
}

fn second_half_forms(s: &mut Vec<char>) {
    let h = B_HASANTA;
    replace(s, &[h, B_BH], &[A_BH_2H]);
    replace(s, &[h, B_T], &[A_T_2H]);
    replace(s, &[h, B_TH], &[A_TH_2H]);
    replace(s, &[h, B_K], &[A_K_2H]);
    loop {
        let i = pos(s, &[h, B_B]);
        if i <= 0 {
            break;
        }
        let c = match at(s, i - 1) {
            Some(B_S | B_SS | B_M | B_N | B_D | A_M_1H | A_SS_1H | A_S_1H_1 | A_N_1H_1 | A_D_1H_2) => A_B_2H_1,
            Some(B_DH | B_B | B_H) => A_B_2H_4,
            Some(B_SH | B_G | B_P) => A_B_2H_3,
            _ => A_B_2H_2,
        };
        stuff(s, i, 2, &[c]);
    }
    loop {
        let i = pos(s, &[h, B_M]);
        if i <= 0 {
            break;
        }
        let c = match at(s, i - 1) {
            Some(A_M_1H | A_SS_1H | A_C_1H | A_S_1H_1 | A_D_1H_2 | A_N_1H_1 | A_N_1H_2) => A_M_2H_2,
            Some(A_NGA_1H) => A_M,
            _ => A_M_2H_1,
        };
        stuff(s, i, 2, &[c]);
    }
    loop {
        let i = pos(s, &[h, B_L]);
        if i <= 0 {
            break;
        }
        let c = if base_line_right(at(s, i - 1)) { A_L_2H_3 } else { A_L_2H_1 };
        stuff(s, i, 2, &[c]);
    }
    replace(s, &[h, B_NN], &[A_NN_2H_1]);
    replace(s, &[h, B_N], &[A_NN_2H_2]);
}

fn consonants(s: &mut Vec<char>) {
    let map = [
        (B_K, A_K), (B_KH, A_KH), (B_G, A_G), (B_GH, A_GH), (B_NGA, A_NGA), (B_C, A_C), (B_CH, A_CH), (B_J, A_J), (B_JH, A_JH),
        (B_NYA, A_NYA), (B_TT, A_TT), (B_TTH, A_TTH), (B_DD, A_DD), (B_DDH, A_DDH), (B_NN, A_NN), (B_T, A_T), (B_TH, A_TH), (B_D, A_D),
        (B_DH, A_DH), (B_N, A_N), (B_P, A_P), (B_PH, A_PH), (B_B, A_B), (B_BH, A_BH), (B_M, A_M), (B_Z, A_Z), (B_R, A_R), (B_L, A_L),
        (B_SH, A_SH), (B_SS, A_SS), (B_S, A_S), (B_H, A_H), (B_Y, A_Y), (B_RR, A_RR), (B_RRH, A_RRH),
    ];
    for c in s.iter_mut() {
        if let Some(&(_, a)) = map.iter().find(|(b, _)| b == c) {
            *c = a;
        }
    }
}

fn final_touch(s: &mut Vec<char>) {
    s.retain(|&c| c != B_HASANTA && c != ZWJ && c != ZWNJ);
    // Avro has six hard-coded pair fixes here; five are unreadable in its repository (stored as U+FFFD) and are left
    // out (see SOURCE.md). The one that survives intact is kept: জু uses the other u-kar glyph.
    replace(s, &[A_J, A_UKAR1], &[A_J, A_UKAR2]);
}

// ---- Bijoy to Unicode ------------------------------------------------------------------------------------------
//
// The reverse of the map above (Likhon's addition, same licence as this file): every Bijoy glyph back to its
// Unicode letters, then the visual order undone. Bijoy writes ি ে ৈ before the consonant cluster and reph after it;
// Unicode wants them the other way round. First-half glyphs become "letter + hasanta", second-half glyphs
// "hasanta + letter", and a doubled hasanta between them collapses to one.

/// A stand-in for reph while the text is reordered (a noncharacter, never in real text).
const REPH_MARK: char = '\u{FDD0}';

fn glyph(c: char) -> Option<&'static str> {
    Some(match c {
        '0'..='9' => return None, // handled by the caller (digits)
        A_TAKA => "৳",
        A_DARI => "।",
        A_DOUBLE_DANDA => "॥",
        A_HASANTA => "্\u{200C}",
        A_START_DOUBLE_QUOTE => "“",
        A_END_DOUBLE_QUOTE => "”",
        A_A => "অ",
        A_I => "ই",
        A_II => "ঈ",
        A_U => "উ",
        A_UU => "ঊ",
        A_RRI => "ঋ",
        A_E => "এ",
        A_OI => "ঐ",
        A_O => "ও",
        A_OU => "ঔ",
        A_AAKAR => "া",
        A_IKAR => "ি",
        A_IIKAR => "ী",
        A_UKAR1 | A_UKAR2 | A_UKAR3 | A_UKAR4 => "ু",
        A_UUKAR1 | A_UUKAR2 | A_UUKAR3 => "ূ",
        A_RRIKAR1 | A_RRIKAR2 => "ৃ",
        A_EKAR1 | A_EKAR2 => "ে",
        A_OIKAR1 | A_OIKAR2 => "ৈ",
        A_OUKAR => "ৗ",
        A_K => "ক",
        A_KH => "খ",
        A_G => "গ",
        A_GH => "ঘ",
        A_NGA => "ঙ",
        A_C => "চ",
        A_CH => "ছ",
        A_J => "জ",
        A_JH => "ঝ",
        A_NYA => "ঞ",
        A_TT => "ট",
        A_TTH => "ঠ",
        A_DD => "ড",
        A_DDH => "ঢ",
        A_NN => "ণ",
        A_T => "ত",
        A_TH => "থ",
        A_D => "দ",
        A_DH => "ধ",
        A_N => "ন",
        A_P => "প",
        A_PH => "ফ",
        A_B => "ব",
        A_BH => "ভ",
        A_M => "ম",
        A_Z => "য",
        A_R => "র",
        A_L => "ল",
        A_SH => "শ",
        A_SS => "ষ",
        A_S => "স",
        A_H => "হ",
        A_RR => "ড়",
        A_RRH => "ঢ়",
        A_Y => "য়",
        A_KHANDATA => "ৎ",
        A_ANUSHAR => "ং",
        A_BISHARGA => "ঃ",
        A_CHANDRA => "ঁ",
        // Full forms
        A_K_K => "ক্ক",
        A_K_TT => "ক্ট",
        A_K_SS_M => "ক্ষ্ম",
        A_K_T => "ক্ত",
        A_K_M => "ক্ম",
        A_K_R => "ক্র",
        A_K_SS => "ক্ষ",
        A_K_S => "ক্স",
        A_G_UKAR => "গু",
        A_G_G => "গ্গ",
        A_G_D => "গ্দ",
        A_G_DH => "গ্ধ",
        A_NGA_K => "ঙ্ক",
        A_NGA_G => "ঙ্গ",
        A_J_J => "জ্জ",
        A_J_JH => "জ্ঝ",
        A_J_NYA => "জ্ঞ",
        A_NYA_C => "ঞ্চ",
        A_NYA_CH => "ঞ্ছ",
        A_NYA_J => "ঞ্জ",
        A_NYA_JH => "ঞ্ঝ",
        A_TT_TT => "ট্ট",
        A_DD_DD => "ড্ড",
        A_NN_TT => "ণ্ট",
        A_NN_TTH => "ণ্ঠ",
        A_NN_DD => "ণ্ড",
        A_T_T => "ত্ত",
        A_T_TH => "ত্থ",
        A_T_M => "ত্ম",
        A_T_R => "ত্র",
        A_D_D => "দ্দ",
        A_D_DH => "দ্ধ",
        A_D_B => "দ্ব",
        A_D_M => "দ্ম",
        A_N_TTH => "ন্ঠ",
        A_N_DD => "ন্ড",
        A_N_DH => "ন্ধ",
        A_N_S => "ন্স",
        A_P_TT => "প্ট",
        A_P_T => "প্ত",
        A_P_P => "প্প",
        A_P_S => "প্স",
        A_B_J => "ব্জ",
        A_B_D => "ব্দ",
        A_B_DH => "ব্ধ",
        A_BH_R => "ভ্র",
        A_M_N => "ম্ন",
        A_M_PH => "ম্ফ",
        A_L_K => "ল্ক",
        A_L_G => "ল্গ",
        A_L_TT => "ল্ট",
        A_L_DD => "ল্ড",
        A_L_P => "ল্প",
        A_L_PH => "ল্ফ",
        A_SH_UKAR => "শু",
        A_SH_C => "শ্চ",
        A_SH_CH => "শ্ছ",
        A_SS_NN => "ষ্ণ",
        A_SS_TT => "ষ্ট",
        A_SS_TTH => "ষ্ঠ",
        A_SS_PH => "ষ্ফ",
        A_S_KH => "স্খ",
        A_S_TT => "স্ট",
        A_S_N => "স্ন",
        A_S_PH => "স্ফ",
        A_H_UKAR => "হু",
        A_H_RRIKAR => "হৃ",
        A_H_N => "হ্ন",
        A_H_M => "হ্ম",
        A_RR_G => "ড়্গ",
        // First half forms (the letter and its hasanta)
        A_M_1H => "ম্",
        A_SS_1H => "ষ্",
        A_S_1H_1 => "স্",
        A_N_1H_1 | A_N_1H_2 => "ন্",
        A_D_1H_1 | A_D_1H_2 => "দ্",
        A_C_1H => "চ্",
        A_NGA_1H => "ঙ্",
        // Second half forms (hasanta and the letter)
        A_B_2H_1 | A_B_2H_2 | A_B_2H_3 | A_B_2H_4 => "্ব",
        A_BH_2H => "্ভ",
        A_BH_R_2H => "্ভ্র",
        A_M_2H_1 | A_M_2H_2 => "্ম",
        A_ZFOLA => "্য",
        A_RFOLA_1 | A_RFOLA_2 | A_RFOLA_3 => "্র",
        A_L_2H_1 | A_L_2H_3 => "্ল",
        A_T_R_2H => "্ত্র",
        A_NN_2H_1 => "্ণ",
        A_K_R_2H => "্ক্র",
        A_NN_2H_2 => "্ন",
        A_T_2H => "্ত",
        A_T_UKAR_2H => "্তু",
        A_TH_2H => "্থ",
        A_K_2H => "্ক",
        _ => return None,
    })
}

fn is_consonant(c: char) -> bool {
    is_pure_consonant(c)
}

/// Converts Bijoy-encoded text (as typed for SutonnyMJ and its family) back to Unicode Bangla.
pub fn bijoy_to_unicode(text: &str) -> String {
    // 1. Glyphs to letters, in Bijoy's visual order.
    let mut s: Vec<char> = Vec::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_digit() {
            s.push(char::from_u32(B_0 as u32 + (c as u32 - '0' as u32)).unwrap());
        } else if c == A_REPH {
            s.push(REPH_MARK);
        } else if let Some(u) = glyph(c) {
            s.extend(u.chars());
        } else {
            s.push(c);
        }
    }
    // A first-half form followed by a second-half form gives a doubled hasanta.
    replace(&mut s, &[B_HASANTA, B_HASANTA], &[B_HASANTA]);

    // 2. Reph: Bijoy puts it after the cluster; it belongs before it, as র + hasanta.
    let mut out: Vec<char> = Vec::with_capacity(s.len());
    for &c in &s {
        if c != REPH_MARK {
            out.push(c);
            continue;
        }
        // Skip back over vowel signs that follow the cluster, then over the cluster (C (্ C)*).
        let mut i = out.len();
        while i > 0 && matches!(out[i - 1], B_AAKAR | B_IIKAR | B_UKAR | B_UUKAR | B_RRIKAR | B_LENGTH_MARK) {
            i -= 1;
        }
        if i > 0 && is_consonant(out[i - 1]) {
            i -= 1;
            while i >= 2 && out[i - 1] == B_HASANTA && is_consonant(out[i - 2]) {
                i -= 2;
            }
        }
        out.splice(i..i, [B_R, B_HASANTA]);
    }

    // 3. Pre-base vowel signs: Bijoy writes ি ে ৈ before the cluster; move each after it.
    let mut s = out;
    let mut out: Vec<char> = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if matches!(c, B_IKAR | B_EKAR | B_OIKAR) {
            let mut j = i + 1;
            // An optional reph (র্) and then C (্ C)*.
            if j + 1 < s.len() && s[j] == B_R && s[j + 1] == B_HASANTA && j + 2 < s.len() && is_consonant(s[j + 2]) {
                j += 2;
            }
            if j < s.len() && is_consonant(s[j]) {
                j += 1;
                while j + 1 < s.len() && s[j] == B_HASANTA && is_consonant(s[j + 1]) {
                    j += 2;
                }
                if j < s.len() && s[j] == B_NUKTA {
                    j += 1;
                }
                out.extend_from_slice(&s[i + 1..j]);
                out.push(c);
                i = j;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    s = out;

    // 4. Two-part vowel signs and the vowel written as two glyphs.
    replace(&mut s, &[B_EKAR, B_AAKAR], &[B_OKAR]);
    replace(&mut s, &[B_EKAR, B_LENGTH_MARK], &[B_OUKAR]);
    replace(&mut s, &[B_A, B_AAKAR], &[B_AA]);
    s.retain(|&c| c != REPH_MARK);
    s.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_vowel_signs() {
        assert_eq!(unicode_to_bijoy("আমি"), "Avwg");
        assert_eq!(unicode_to_bijoy("বাংলা"), "evsjv");
        // An e-kar or oi-kar starting a word uses the first glyph, inside a word the second (as Avro does).
        assert_eq!(unicode_to_bijoy("কে"), "†K");
        assert_eq!(unicode_to_bijoy("দেশে"), "†`‡k");
        assert_eq!(unicode_to_bijoy("কৈ"), "ˆK");
        assert_eq!(unicode_to_bijoy("কো"), "†Kv");
        assert_eq!(unicode_to_bijoy("কৌ"), "†KŠ");
        assert_eq!(unicode_to_bijoy("১২৩ ৳ ।"), "123 $ |");
    }

    #[test]
    fn conjuncts_reph_and_folas() {
        assert_eq!(unicode_to_bijoy("ক্ষ"), "¶");
        assert_eq!(unicode_to_bijoy("ক্ত"), "³");
        assert_eq!(unicode_to_bijoy("কর্ম"), "Kg©");
        assert_eq!(unicode_to_bijoy("প্র"), "cÖ");
        assert_eq!(unicode_to_bijoy("ক্য"), "K¨");
        assert_eq!(unicode_to_bijoy("ক্তি"), "w³");
    }

    #[test]
    fn a_sentence() {
        assert_eq!(unicode_to_bijoy("আমি বাংলায় গান গাই।"), "Avwg evsjvq Mvb MvB|");
    }

    #[test]
    fn bijoy_back_to_unicode() {
        assert_eq!(bijoy_to_unicode("Avwg evsjvq Mvb MvB|"), "আমি বাংলায় গান গাই।");
        assert_eq!(bijoy_to_unicode("†`‡k"), "দেশে");
        assert_eq!(bijoy_to_unicode("Kg©"), "কর্ম");
        assert_eq!(bijoy_to_unicode("†Kv"), "কো");
    }

    #[test]
    fn round_trip() {
        for w in [
            "আমি বাংলায় গান গাই।", "কর্ম", "কর্মী", "ধর্মের", "বিজ্ঞান", "ক্ষমা", "স্কুল", "প্রেম", "শ্রী", "কৃষক", "রুপা", "সূর্য",
            "দুঃখ", "চাঁদ", "ব্যাকরণ", "ছাত্র", "বৈশাখ", "কৌতুক", "মৌমাছি", "১২৩৪৫৬৭৮৯০ ৳", "উৎসব", "ঋষি", "ঐক্য", "স্নান", "আত্মা",
        ] {
            assert_eq!(bijoy_to_unicode(&unicode_to_bijoy(w)), w, "{w} -> {}", unicode_to_bijoy(w));
        }
    }

    #[test]
    fn non_bangla_passes_through() {
        assert_eq!(unicode_to_bijoy("Likhon 2026"), "Likhon 2026");
        assert_eq!(unicode_to_bijoy(""), "");
    }
}
