// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MIT
//! Khipro, the compositional Bangla layout (khipro.khiproteam.com), typed from its own m17n input-method file.
//!
//! `bn-khipro-classic.mim` is the Khipro team's file, unchanged (MIT, LICENSE-khipro-m17n). The interpreter below
//! is a Rust port of the Khipro team's cross-platform engine (khipro-library core/src/engine.cpp, MIT,
//! LICENSE-khipro-library): the same subset of m17n (maps, states with entry actions, set, insert, delete, move,
//! mark, shift, cond, undo), the same longest-match rule across a state's maps, and the same whole-word conversion
//! each time a key is added. Reading the .mim text (the library compiles it to C++ tables instead) is Likhon's own.
//! SOURCE.md names the exact commits.

use crate::Action;
use std::collections::HashMap;
#[cfg(feature = "bundled-khipro")]
use std::sync::OnceLock;

/// The Khipro Classic input method (MIT), built in only with the `bundled-khipro` feature (the website's WebAssembly
/// and the tests). likhon.exe and the keyboard DLL leave it out: Khipro is added from the library (owner, 2026-10-23).
#[cfg(feature = "bundled-khipro")]
pub const KHIPRO_CLASSIC: &str = include_str!("bn-khipro-classic.mim");

// ---- The file: s-expressions ------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Sx {
    List(Vec<Sx>),
    Str(String),
    Sym(String),
}

fn parse_sx(src: &str) -> Vec<Sx> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut stack: Vec<Vec<Sx>> = vec![Vec::new()];
    while i < chars.len() {
        let c = chars[i];
        match c {
            ';' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '(' => {
                stack.push(Vec::new());
                i += 1;
            }
            ')' => {
                let done = stack.pop().unwrap_or_default();
                if let Some(top) = stack.last_mut() {
                    top.push(Sx::List(done));
                }
                i += 1;
            }
            '"' => {
                let mut s = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                    }
                    s.push(chars[i]);
                    i += 1;
                }
                i += 1;
                if let Some(top) = stack.last_mut() {
                    top.push(Sx::Str(s));
                }
            }
            '?' if i + 1 < chars.len() && !chars[i + 1].is_whitespace() && chars[i + 1] != ')' => {
                // m17n character literal: ?X is the character X.
                let ch = if chars[i + 1] == '\\' && i + 2 < chars.len() { i += 1; chars[i + 1] } else { chars[i + 1] };
                i += 2;
                if let Some(top) = stack.last_mut() {
                    top.push(Sx::Str(ch.to_string()));
                }
            }
            c if c.is_whitespace() => i += 1,
            _ => {
                let start = i;
                while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '(' && chars[i] != ')' {
                    i += 1;
                }
                if let Some(top) = stack.last_mut() {
                    top.push(Sx::Sym(chars[start..i].iter().collect()));
                }
            }
        }
    }
    stack.into_iter().next().unwrap_or_default()
}

// ---- The input method ------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Cond {
    Always,
    Eq(String, i32),
    Gt(String, i32),
    And(Box<Cond>, Box<Cond>),
    Or(Box<Cond>, Box<Cond>),
}

#[derive(Debug, Clone)]
enum Act {
    Set(String, String),
    Insert(Vec<char>),
    Delete(usize),
    Move(i32),
    MoveEnd,
    Mark(String),
    MoveToMark(String),
    DeleteMark(String),
    Shift(String),
    Undo,
    Cond(Vec<(Cond, Vec<Act>)>),
}

#[derive(Debug, Default)]
struct State {
    entry: Vec<Act>,
    rules: Vec<(String, Vec<Act>)>,
}

#[derive(Debug, Default)]
pub struct InputMethod {
    maps: HashMap<String, Vec<(Vec<char>, Vec<Act>)>>,
    states: HashMap<String, State>,
}

fn num(s: &str) -> Option<i32> {
    s.parse().ok()
}

fn cond(x: &Sx) -> Cond {
    match x {
        Sx::Sym(s) if s == "1" || s == "t" => Cond::Always,
        Sx::List(l) => match l.as_slice() {
            [Sx::Sym(op), Sx::Sym(v), Sx::Sym(n)] if op == "=" => Cond::Eq(v.clone(), num(n).unwrap_or(0)),
            [Sx::Sym(op), Sx::Sym(v), Sx::Sym(n)] if op == ">" => Cond::Gt(v.clone(), num(n).unwrap_or(0)),
            [Sx::Sym(op), a, b] if op == "&" => Cond::And(Box::new(cond(a)), Box::new(cond(b))),
            [Sx::Sym(op), a, b] if op == "|" => Cond::Or(Box::new(cond(a)), Box::new(cond(b))),
            [Sx::Sym(op), a, rest @ ..] if (op == "&" || op == "|") && !rest.is_empty() => {
                // More than two operands: fold left.
                let mut c = cond(a);
                for r in rest {
                    c = if op == "&" { Cond::And(Box::new(c), Box::new(cond(r))) } else { Cond::Or(Box::new(c), Box::new(cond(r))) };
                }
                c
            }
            _ => Cond::Always,
        },
        _ => Cond::Always,
    }
}

/// `@-2` and friends: positions relative to the cursor.
fn rel(s: &str) -> Option<i32> {
    match s {
        "@-" => Some(-1),
        "@+" => Some(1),
        _ => s.strip_prefix('@').and_then(num),
    }
}

fn acts(xs: &[Sx]) -> Vec<Act> {
    xs.iter().filter_map(act).collect()
}

fn act(x: &Sx) -> Option<Act> {
    match x {
        Sx::Str(s) => Some(Act::Insert(s.chars().collect())),
        Sx::List(l) => {
            let (Sx::Sym(head), rest) = (l.first()?, &l[1..]) else { return None };
            let arg = |i: usize| match rest.get(i) {
                Some(Sx::Sym(s)) | Some(Sx::Str(s)) => Some(s.clone()),
                _ => None,
            };
            Some(match head.as_str() {
                "set" => Act::Set(arg(0)?, arg(1)?),
                "insert" => Act::Insert(arg(0)?.chars().collect()),
                "delete" => {
                    let a = arg(0)?;
                    match rel(&a) {
                        Some(n) if n < 0 => Act::Delete((-n) as usize),
                        Some(_) => return None,
                        None => Act::DeleteMark(a),
                    }
                }
                "move" => {
                    let a = arg(0)?;
                    match a.as_str() {
                        "@>" => Act::MoveEnd,
                        "@<" => Act::Move(i32::MIN / 2),
                        _ => rel(&a).map(Act::Move).unwrap_or(Act::MoveToMark(a)),
                    }
                }
                "mark" => Act::Mark(arg(0)?),
                "shift" => Act::Shift(arg(0)?),
                "undo" => Act::Undo,
                "commit" => return None, // the agent commits on Space; inside a word it changes nothing
                "cond" => Act::Cond(rest.iter().filter_map(|b| match b {
                    Sx::List(bl) if !bl.is_empty() => Some((cond(&bl[0]), acts(&bl[1..]))),
                    _ => None,
                }).collect()),
                _ => return None,
            })
        }
        Sx::Sym(_) => None,
    }
}

impl InputMethod {
    /// Reads an m17n input method: its `map` and `state` sections. Keys that are not text (BackSpace) are skipped:
    /// the agent ends the word on those keys itself.
    pub fn parse(src: &str) -> InputMethod {
        let mut im = InputMethod::default();
        for top in parse_sx(src) {
            let Sx::List(l) = top else { continue };
            match l.first() {
                Some(Sx::Sym(s)) if s == "map" => {
                    for m in &l[1..] {
                        let Sx::List(ml) = m else { continue };
                        let Some(Sx::Sym(name)) = ml.first() else { continue };
                        let entries = ml[1..].iter().filter_map(|e| match e {
                            Sx::List(el) => match el.first() {
                                Some(Sx::Str(k)) if !k.is_empty() => Some((k.chars().collect(), acts(&el[1..]))),
                                _ => None,
                            },
                            _ => None,
                        });
                        im.maps.entry(name.clone()).or_default().extend(entries);
                    }
                }
                Some(Sx::Sym(s)) if s == "state" => {
                    for st in &l[1..] {
                        let Sx::List(sl) = st else { continue };
                        let Some(Sx::Sym(name)) = sl.first() else { continue };
                        let mut state = State::default();
                        for r in &sl[1..] {
                            let Sx::List(rl) = r else { continue };
                            match rl.first() {
                                Some(Sx::Sym(m)) if m == "t" => state.entry = acts(&rl[1..]),
                                Some(Sx::Sym(m)) => state.rules.push((m.clone(), acts(&rl[1..]))),
                                _ => {}
                            }
                        }
                        im.states.insert(name.clone(), state);
                    }
                }
                _ => {}
            }
        }
        im
    }

    /// Every character any map key uses: the keys the layout takes.
    pub fn takes(&self, c: char) -> bool {
        self.maps.values().flatten().any(|(k, _)| k.contains(&c))
    }

    /// What typing `input` gives, from the start of a word.
    pub fn convert(&self, input: &str) -> String {
        let input: Vec<char> = input.chars().collect();
        let mut run = Run { im: self, out: Vec::new(), cursor: 0, vars: HashMap::new(), marks: HashMap::new(), state: "init".into() };
        run.enter("init");
        let mut i = 0;
        let mut guard = 0;
        while i < input.len() && guard < input.len() * 4 + 16 {
            guard += 1;
            let Some(st) = self.states.get(&run.state) else { break };
            match self.longest(st, &input, i) {
                Some((key_len, entry, rule)) => {
                    let silent = !entry.iter().any(|a| matches!(a, Act::Insert(_)));
                    run.exec(entry, false);
                    run.exec(rule, silent);
                    run.cursor = run.cursor.min(run.out.len());
                    i += key_len;
                }
                None if run.state != "init" => {
                    // A key the state has no rule for starts a new composition in init (engine.cpp does the same).
                    run.state = "init".into();
                    run.enter("init");
                }
                None => {
                    run.out.insert(run.cursor, input[i]);
                    run.shift_marks_insert(run.cursor, 1);
                    run.cursor += 1;
                    i += 1;
                }
            }
        }
        run.out.into_iter().collect()
    }

    fn longest<'a>(&'a self, st: &'a State, input: &[char], at: usize) -> Option<(usize, &'a [Act], &'a [Act])> {
        let mut best: Option<(usize, &[Act], &[Act])> = None;
        for (map, rule) in &st.rules {
            for (key, entry) in self.maps.get(map).into_iter().flatten() {
                if key.len() > best.map_or(0, |b| b.0) && input[at..].starts_with(key) {
                    best = Some((key.len(), entry, rule));
                }
            }
        }
        best
    }
}

struct Run<'a> {
    im: &'a InputMethod,
    out: Vec<char>,
    cursor: usize,
    vars: HashMap<String, i32>,
    marks: HashMap<String, usize>,
    state: String,
}

impl Run<'_> {
    fn enter(&mut self, state: &str) {
        if let Some(st) = self.im.states.get(state) {
            self.exec(&st.entry, false);
        }
    }

    fn shift_marks_insert(&mut self, pos: usize, len: usize) {
        self.marks.values_mut().filter(|at| **at > pos).for_each(|at| *at += len);
    }

    fn shift_marks_delete(&mut self, start: usize, end: usize) {
        for at in self.marks.values_mut() {
            if *at >= end {
                *at -= end - start;
            } else if *at > start {
                *at = start;
            }
        }
    }

    fn test(&self, c: &Cond) -> bool {
        let var = |v: &String| self.vars.get(v).copied().unwrap_or(0);
        match c {
            Cond::Always => true,
            Cond::Eq(v, n) => var(v) == *n,
            Cond::Gt(v, n) => var(v) > *n,
            Cond::And(a, b) => self.test(a) && self.test(b),
            Cond::Or(a, b) => self.test(a) || self.test(b),
        }
    }

    fn exec(&mut self, actions: &[Act], silent: bool) {
        for a in actions {
            match a {
                Act::Set(v, val) => {
                    let n = self.vars.get(val).copied().or_else(|| num(val)).unwrap_or(0);
                    self.vars.insert(v.clone(), n);
                }
                Act::Insert(text) => {
                    let pos = self.cursor.min(self.out.len());
                    self.out.splice(pos..pos, text.iter().copied());
                    self.shift_marks_insert(pos, text.len());
                    self.cursor = pos + text.len();
                }
                Act::Delete(n) => {
                    let end = self.cursor.min(self.out.len());
                    let start = end.saturating_sub(*n);
                    if start < end {
                        self.out.drain(start..end);
                        self.shift_marks_delete(start, end);
                        self.cursor = start;
                    }
                }
                Act::Move(n) => self.cursor = (self.cursor as i64 + *n as i64).clamp(0, self.out.len() as i64) as usize,
                Act::MoveEnd => self.cursor = self.out.len(),
                Act::Mark(m) => {
                    self.marks.insert(m.clone(), self.cursor);
                }
                Act::MoveToMark(m) => self.cursor = self.marks.get(m).copied().unwrap_or(0).min(self.out.len()),
                Act::DeleteMark(m) => {
                    if let Some(&at) = self.marks.get(m) {
                        let (start, end) = (at.min(self.cursor), at.max(self.cursor).min(self.out.len()));
                        if start < end {
                            self.out.drain(start..end);
                            self.shift_marks_delete(start, end);
                            self.cursor = start;
                        }
                    }
                }
                Act::Shift(target) => {
                    // engine.cpp's silent-vowel rule: an entry that typed nothing (the inherent vowel) ends a cluster.
                    let target = if silent && target == "fusion_possible" && self.im.states.contains_key("shor_state") { "shor_state" } else { target.as_str() };
                    self.state = target.to_string();
                    let t = self.state.clone();
                    self.enter(&t);
                }
                Act::Undo => {
                    if self.cursor > 0 && !self.out.is_empty() {
                        let at = (self.cursor - 1).min(self.out.len() - 1);
                        self.out.remove(at);
                        self.cursor = at;
                    }
                }
                Act::Cond(branches) => {
                    if let Some((_, body)) = branches.iter().find(|(c, _)| self.test(c)) {
                        let body = body.clone();
                        self.exec(&body, silent);
                    }
                }
            }
        }
    }
}

/// Khipro Classic, parsed once.
#[cfg(feature = "bundled-khipro")]
pub fn classic() -> &'static InputMethod {
    static IM: OnceLock<InputMethod> = OnceLock::new();
    IM.get_or_init(|| InputMethod::parse(KHIPRO_CLASSIC))
}

/// Typing Khipro one key at a time: the word so far is converted whole after each key, and only the part that
/// changed on screen is retyped. The input method is the Khipro file the person added (or the bundled one).
pub struct Khipro {
    im: std::sync::Arc<InputMethod>,
    tape: String,
    shown: String,
}

impl Khipro {
    pub fn new(im: std::sync::Arc<InputMethod>) -> Self {
        Self { im, tape: String::new(), shown: String::new() }
    }

    pub fn key(&mut self, ch: char) -> Action {
        let im = &self.im;
        if ch.is_whitespace() || !ch.is_ascii_graphic() || !im.takes(ch) {
            self.reset();
            return Action::PassThrough;
        }
        self.tape.push(ch);
        let now = im.convert(&self.tape);
        let same = self.shown.chars().zip(now.chars()).take_while(|(a, b)| a == b).count();
        let backspaces = self.shown.chars().skip(same).map(char::len_utf16).sum();
        let output: String = now.chars().skip(same).collect();
        self.shown = now;
        Action::Replace { backspaces, output }
    }

    /// The word ends (Space, Enter, a click): the next key starts a new composition.
    pub fn reset(&mut self) {
        self.tape.clear();
        self.shown.clear();
    }
}

#[cfg(all(test, feature = "bundled-khipro"))]
mod tests {
    use super::*;

    #[test]
    fn the_khipro_team_test_cases_pass() {
        let im = classic();
        let mut failed = Vec::new();
        let cases: Vec<(&str, &str)> = include_str!("khipro-testcases.tsv").lines().skip(1).filter(|l| !l.trim().is_empty()).filter_map(|l| l.split_once('\t')).collect();
        for (input, want) in &cases {
            let got = im.convert(input);
            if got != *want {
                failed.push(format!("{input:?}: want {want:?}, got {got:?}"));
            }
        }
        assert!(failed.is_empty(), "{} of {} failed:\n{}", failed.len(), cases.len(), failed.join("\n"));
    }

    #[test]
    fn typing_key_by_key_matches_converting_the_word() {
        let im = std::sync::Arc::new(InputMethod::parse(KHIPRO_CLASSIC));
        for word in ["ami", "bangla", "kfr", "ks/", "shikkha"] {
            let mut k = Khipro::new(im.clone());
            let mut screen = String::new();
            for c in word.chars() {
                if let Action::Replace { backspaces, output } = k.key(c) {
                    let mut u: Vec<u16> = screen.encode_utf16().collect();
                    u.truncate(u.len() - backspaces);
                    screen = String::from_utf16(&u).unwrap() + &output;
                }
            }
            assert_eq!(screen, classic().convert(word), "{word}");
        }
    }
}
