// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! Layouts and themes added from a file (v2 §12 sideload; Desktop/CLAUDE.md 7: layouts and themes only, strict
//! parse, size caps, a fixed set of allowed fields, a warning that the file was not reviewed, and removable).
//!
//! The files are JSON only (shared/schemas/layout.schema.json, theme.schema.json). An accepted file is copied into
//! the data folder (layouts/<id>.json, themes/<id>.json) under its own id; ids of built-in layouts and themes are
//! refused so a file can never stand in for one of them.

use crate::themes::{Theme, THEMES};
use likhon_engine::fixed::{Layout, MAX_LAYOUT_BYTES};
use serde_json::Value;
use std::path::PathBuf;

const MAX_THEME_BYTES: usize = 16 * 1024;
const COLORS: [&str; 9] = ["desk", "surface", "surface2", "line", "ink", "muted", "accent", "on", "sel"];

fn folder(kind: &str) -> PathBuf {
    crate::settings::dir().0.join(kind)
}

/// Checks a theme file: the schema's fields only, both versions, nine colours each, #RRGGBB or a known token.
pub fn check_theme(text: &str) -> Result<(String, String), String> {
    if text.len() > MAX_THEME_BYTES {
        return Err("the theme file is too large".into());
    }
    let v: Value = serde_json::from_str(text).map_err(|e| format!("not valid JSON: {e}"))?;
    let o = v.as_object().ok_or("not a JSON object")?;
    const FIELDS: [&str; 8] = ["format", "version", "id", "name", "author", "license", "light", "dark"];
    if let Some(k) = o.keys().find(|k| !FIELDS.contains(&k.as_str())) {
        return Err(format!("unknown field \"{k}\""));
    }
    if o.get("format").and_then(|f| f.as_str()) != Some("likhon-theme") || o.get("version").and_then(|v| v.as_u64()) != Some(1) {
        return Err("not a Likhon theme, version 1".into());
    }
    let id = o.get("id").and_then(|v| v.as_str()).filter(|id| !id.is_empty() && id.len() <= 40 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')).ok_or("id must be lower-case letters, digits and hyphens")?;
    let name = o.get("name").and_then(|n| n.get("en")).and_then(|n| n.as_str()).filter(|n| !n.is_empty() && n.chars().count() <= 60).ok_or("name.en missing")?;
    for mode in ["light", "dark"] {
        let c = o.get(mode).and_then(|c| c.as_object()).ok_or_else(|| format!("{mode} missing: every theme has a light and a dark version"))?;
        if let Some(k) = c.keys().find(|k| !COLORS.contains(&k.as_str())) {
            return Err(format!("{mode}: unknown colour \"{k}\""));
        }
        for k in COLORS {
            let s = c.get(k).and_then(|x| x.as_str()).ok_or_else(|| format!("{mode}.{k} missing"))?;
            if color(s, mode == "dark").is_none() {
                return Err(format!("{mode}.{k}: use #RRGGBB or a design token like @accent"));
            }
        }
    }
    Ok((id.to_string(), name.to_string()))
}

fn color(s: &str, dark: bool) -> Option<u32> {
    match s.strip_prefix('@') {
        Some(name) => crate::tokens::lookup(name, dark),
        None => (s.len() == 7 && s.starts_with('#')).then(|| u32::from_str_radix(&s[1..], 16).ok()).flatten(),
    }
}

/// Validates and stores a file. Returns (kind, id, name).
pub fn add(kind: &str, text: &str) -> Result<(String, String), String> {
    let (id, name) = match kind {
        "layouts" => {
            if text.len() > MAX_LAYOUT_BYTES {
                return Err("the layout file is too large".into());
            }
            let l = Layout::from_json(text)?;
            if l.id == "phonetic" || crate::layouts::BUILT_IN.iter().any(|(i, _)| *i == l.id) {
                return Err(format!("\"{}\" is a built-in layout; give the file its own id", l.id));
            }
            (l.id, l.name_en)
        }
        "themes" => {
            let (id, name) = check_theme(text)?;
            if THEMES.iter().any(|t| t.id == id) {
                return Err(format!("\"{id}\" is a built-in theme; give the file its own id"));
            }
            (id, name)
        }
        _ => return Err("only layouts and themes can be added from a file".into()),
    };
    let dir = folder(kind);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{id}.json")), text).map_err(|e| e.to_string())?;
    Ok((id, name))
}

pub fn remove(kind: &str, id: &str) -> bool {
    if !matches!(kind, "layouts" | "themes") || !id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
        return false;
    }
    let json = std::fs::remove_file(folder(kind).join(format!("{id}.json"))).is_ok();
    // An input method added from the library (layouts/<id>.mim).
    let mim = kind == "layouts" && std::fs::remove_file(folder(kind).join(format!("{id}.mim"))).is_ok();
    json || mim
}

/// Input methods added from the library (layouts/<id>.mim): (id, English name, Bangla name). The names come from
/// library.json (what the library called the item), else the id.
pub fn input_methods() -> Vec<(String, String, Option<String>)> {
    let Ok(rd) = std::fs::read_dir(folder("layouts")) else { return Vec::new() };
    let names: Value = std::fs::read_to_string(crate::settings::dir().0.join("library.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    rd.flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.extension()? != "mim" {
                return None;
            }
            let id = p.file_stem()?.to_string_lossy().to_string();
            let n = &names[&id]["name"];
            Some((id.clone(), n["en"].as_str().unwrap_or(&id).to_string(), n["bn"].as_str().map(String::from)))
        })
        .collect()
}

/// Stores an input method added from the library (an m17n .mim file, such as Khipro's) as layouts/<id>.mim, after
/// checking that the engine reads it as one.
pub fn add_input_method(id: &str, text: &str) -> Result<(), String> {
    if text.len() > 512 * 1024 || !id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') || id.is_empty() {
        return Err("not an input method this app can use".into());
    }
    let im = likhon_engine::mit::khipro::InputMethod::parse(text);
    if !im.takes('a') {
        return Err("not an input method this app can use".into());
    }
    let dir = folder("layouts");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{id}.mim")), text).map_err(|e| e.to_string())
}

/// The added files of a kind that still pass the checks: (id, the file's JSON).
pub fn list(kind: &str) -> Vec<(String, Value)> {
    let Ok(rd) = std::fs::read_dir(folder(kind)) else { return Vec::new() };
    let mut out: Vec<(String, Value)> = rd
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            let text = std::fs::read_to_string(e.path()).ok()?;
            let ok = match kind {
                "layouts" => Layout::from_json(&text).ok().map(|l| l.id),
                _ => check_theme(&text).ok().map(|x| x.0),
            }?;
            Some((ok, serde_json::from_str(&text).ok()?))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// An added layout by id, for typing.
pub fn layout(id: &str) -> Option<Layout> {
    let text = std::fs::read_to_string(folder("layouts").join(format!("{id}.json"))).ok()?;
    Layout::from_json(&text).ok()
}

/// An added theme by id, in the light or dark version, for the top bar and suggestion window.
pub fn theme(id: &str, dark: bool) -> Option<Theme> {
    let text = std::fs::read_to_string(folder("themes").join(format!("{id}.json"))).ok()?;
    check_theme(&text).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let c = &v[if dark { "dark" } else { "light" }];
    let get = |k: &str| c[k].as_str().and_then(|s| color(s, dark));
    Some(Theme {
        // A handful of themes at most, read when the theme changes: leaking the short id is fine.
        id: Box::leak(id.to_string().into_boxed_str()),
        dark,
        bg: get("surface")?,
        border: get("line")?,
        accent: get("accent")?,
        on_accent: get("on")?,
        text: get("ink")?,
        text2: get("muted")?,
        sel: get("sel")?,
        soft: get("surface2")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_files_are_checked_strictly() {
        let ok = include_str!("../../assets/themes/paper.json").replace("\"paper\"", "\"my-paper\"");
        assert_eq!(check_theme(&ok).unwrap().0, "my-paper");
        for bad in [
            ok.replace("\"dark\": {", "\"night\": {"),
            ok.replace("\"#FBF8F1\"", "\"red\""),
            ok.replace("\"author\"", "\"script\""),
            ok.replace("\"likhon-theme\"", "\"other\""),
        ] {
            assert!(check_theme(&bad).is_err(), "{bad}");
        }
        assert!(check_theme(&include_str!("../../assets/themes/default.json")).is_ok(), "token colours");
    }
}
