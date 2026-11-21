//! What Settings fetches from the Likhon site, always because the person clicked something (net.rs has the rules):
//! the latest release (Updates), the library and adding an item from it (Library), the dictionary for a copy without
//! one (Updates), and donating words (Privacy). Runs on a worker thread of the Settings process; the typing agent is
//! told to reload what changed.
//!
//! Library items are checked against the SHA-256 the site lists before anything is added. Library files are signed
//! offline before release; the development site's are not, so an unsigned item is added like a sideloaded file (with
//! the same warning shown first by the page).
//!
//! Added items are remembered in library.json in the data folder (id, type, version, and what to undo), so the page
//! can show them as added and remove them again.

use crate::net;
use likhon_store::{List, UserData};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

fn data(file: &str) -> PathBuf {
    crate::settings::dir().0.join(file)
}

/// What was added from the library (library.json), for the Library page without going online.
pub fn added() -> Value {
    Value::Object(installed())
}

fn installed() -> Map<String, Value> {
    std::fs::read_to_string(data("library.json")).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

fn save_installed(m: &Map<String, Value>) {
    let _ = std::fs::write(data("library.json"), serde_json::to_string_pretty(m).unwrap_or_default());
}

fn json_from(url: &str) -> Result<Value, String> {
    let body = net::get(url)?;
    serde_json::from_slice(&body).map_err(|_| "the site sent something unexpected".into())
}

// ---- Updates ---------------------------------------------------------------------------------------------------

/// The newest release on the site and whether it is newer than this exe, and the same for the dictionary: the site
/// publishes a new one when reviewers have approved new words (a numbered snapshot, "2026.11"). Asked only on a click.
pub fn latest() -> Value {
    match json_from(&format!("{}/api/app/latest", net::site())) {
        Ok(v) => {
            let theirs = v["version"].as_str().unwrap_or("").to_string();
            json!({ "ok": true, "version": theirs, "newer": !theirs.is_empty() && newer(&theirs, env!("CARGO_PKG_VERSION")),
                    "notes": v["notes"], "download": v["download"], "dict": dictionary_latest() })
        }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// The newest dictionary on the site, and whether it is newer than the one beside likhon.exe (a site snapshot is newer
/// than the base file; between snapshots, the higher version).
fn dictionary_latest() -> Value {
    let Ok(info) = json_from(&format!("{}/api/app/dictionary", net::site())) else { return Value::Null };
    let theirs = info["version"].as_str().unwrap_or("");
    let ours = crate::settings::dictionary_path().and_then(|p| likhon_store::Dictionary::open(&p).ok()).and_then(|d| d.info().snapshot);
    let is_newer = !theirs.is_empty() && theirs != "base" && ours.as_deref().is_none_or(|o| newer(theirs, o));
    json!({ "version": theirs, "words": info["words"], "newer": is_newer })
}

/// Whether version `a` is newer than `b` ("0.10.1" > "0.9.3"), compared number by number.
pub fn newer(a: &str, b: &str) -> bool {
    let parts = |s: &str| s.split(['.', '-']).map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    parts(a) > parts(b)
}

// ---- Library ---------------------------------------------------------------------------------------------------

/// The site's library, with what is added here marked.
pub fn library() -> Value {
    let have = installed();
    match json_from(&format!("{}/api/app/library", net::site())) {
        Ok(v) => {
            let items: Vec<Value> = v["items"].as_array().cloned().unwrap_or_default().into_iter().map(|mut it| {
                let id = it["id"].as_str().unwrap_or("").to_string();
                it["added"] = have.get(&id).map(|h| h["version"].clone()).unwrap_or(Value::Null);
                it["can"] = Value::Bool(matches!(it["type"].as_str(), Some("layout" | "theme" | "wordlist" | "autocorrect" | "shortcuts" | "font")));
                it
            }).collect();
            json!({ "ok": true, "items": items })
        }
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// Downloads one library item, checks it, and adds it by type.
pub fn add(item: &Value) -> Value {
    match add_inner(item) {
        Ok(msg) => json!({ "ok": true, "added": msg }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

fn add_inner(item: &Value) -> Result<String, String> {
    let id = item["id"].as_str().filter(|s| s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')).ok_or("bad item")?.to_string();
    let url = item["file"].as_str().ok_or("bad item")?;
    let body = net::get(url)?;
    let want = item["sha256"].as_str().unwrap_or("");
    if want.is_empty() || net::sha256_hex(&body) != want.to_ascii_lowercase() {
        return Err("the file does not match the library (checksum)".into());
    }
    let kind = item["type"].as_str().unwrap_or("");
    let mut undo = json!({});
    match kind {
        "layout" if item["file_name"].as_str().is_some_and(|f| f.ends_with(".mim")) => {
            // An input method (Khipro): typed by the engine's m17n interpreter.
            let text = String::from_utf8(body).map_err(|_| "not a text file")?;
            let lid = id.to_ascii_lowercase();
            crate::sideload::add_input_method(&lid, &text)?;
            undo = json!({ "sideload": "layouts", "id": lid });
        }
        "layout" | "theme" => {
            let text = String::from_utf8(body).map_err(|_| "not a text file")?;
            let text = library_format(kind, &id, &text)?;
            let (sid, _) = crate::sideload::add(&format!("{kind}s"), &text)?;
            undo = json!({ "sideload": format!("{kind}s"), "id": sid });
        }
        "wordlist" | "autocorrect" | "shortcuts" => {
            let text = String::from_utf8(body).map_err(|_| "not a text file")?;
            let list = match kind { "wordlist" => List::Words, "autocorrect" => List::Autocorrect, _ => List::Shortcuts };
            let db = UserData::open(&likhon_store::user_db(&crate::settings::dir().0))?;
            let before: std::collections::HashSet<String> = db.all(list).into_iter().map(|e| e.key).collect();
            // The library's lists are CSV ("from,to" with a header) or one word per line; user.sqlite takes tab-separated.
            let rows: String = text.lines().skip_while(|l| l.trim_start_matches('\u{feff}').trim().eq_ignore_ascii_case("from,to"))
                .map(|l| l.splitn(2, ',').map(str::trim).collect::<Vec<_>>().join("\t")).collect::<Vec<_>>().join("\n");
            db.import(list, &rows);
            let added: Vec<String> = db.all(list).into_iter().map(|e| e.key).filter(|k| !before.contains(k)).collect();
            undo = json!({ "list": kind, "keys": added });
        }
        "font" => {
            let files = fonts::install_zip(&body)?;
            undo = json!({ "fonts": files });
        }
        _ => return Err("this kind of item is added from the website".into()),
    }
    let mut have = installed();
    have.insert(id.clone(), json!({ "type": kind, "version": item["version"], "name": item["name"], "undo": undo }));
    save_installed(&have);
    crate::settings_app::notify_agent(crate::settings_app::MSG_USER_CHANGED, 0);
    crate::settings_app::notify_agent(crate::settings_app::MSG_SETTINGS_CHANGED, 0);
    Ok(id)
}

/// The website's library keeps layouts and themes in its own shape (a version string, colours under "tokens" with five
/// names, no id or kind); the app's files follow shared/schemas. A file already in the app's shape passes through; one
/// in the site's shape is rewritten: the item's id, `kind: fixed` and version 1 for layouts (the site's version kept as
/// layoutVersion), and for themes the five colours spread over the app's nine. Unknown fields are left out, so the
/// strict checks in sideload.rs still decide.
pub fn library_format(kind: &str, id: &str, text: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("not valid JSON: {e}"))?;
    let o = v.as_object().ok_or("not a JSON object")?;
    if o.get("version").and_then(Value::as_u64) == Some(1) {
        return Ok(text.to_string());
    }
    let id = id.to_ascii_lowercase();
    let pick = |keys: &[&str], out: &mut Map<String, Value>| {
        for k in keys {
            if let Some(x) = o.get(*k) {
                out.insert((*k).to_string(), x.clone());
            }
        }
    };
    let mut out = Map::new();
    match kind {
        "layout" => {
            out.insert("format".into(), json!("likhon-layout"));
            out.insert("version".into(), json!(1));
            out.insert("id".into(), json!(id));
            out.insert("kind".into(), json!("fixed"));
            pick(&["name", "author", "description", "license", "source", "keys", "numpad"], &mut out);
            if let Some(ver) = o.get("version").and_then(Value::as_str) {
                out.insert("layoutVersion".into(), json!(ver));
            }
        }
        _ => {
            out.insert("format".into(), json!("likhon-theme"));
            out.insert("version".into(), json!(1));
            out.insert("id".into(), json!(id));
            pick(&["name", "author", "license"], &mut out);
            for mode in ["light", "dark"] {
                let t = &o.get("tokens").and_then(|t| t.get(mode)).cloned().unwrap_or(Value::Null);
                let c = |k: &str| t.get(k).and_then(Value::as_str).and_then(hex);
                let (Some(bg), Some(border), Some(accent), Some(ink)) = (c("background"), c("border"), c("accent"), c("text")) else {
                    return Err(format!("the theme has no {mode} colours"));
                };
                let sel = c("selected-row-bg").unwrap_or_else(|| mix(accent, bg, 0.2));
                // Black or white on the accent, whichever reads better.
                let on = if luminance(accent) > 0.4 { 0x000000 } else { 0xFFFFFF };
                out.insert(mode.into(), json!({
                    "desk": css(bg), "surface": css(bg), "surface2": css(mix(border, bg, 0.35)), "line": css(border),
                    "ink": css(ink), "muted": css(mix(ink, bg, 0.62)), "accent": css(accent), "on": css(on), "sel": css(sel),
                }));
            }
        }
    }
    serde_json::to_string_pretty(&Value::Object(out)).map_err(|e| e.to_string())
}

fn hex(s: &str) -> Option<u32> {
    let h = s.strip_prefix('#')?;
    (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok()).flatten()
}

fn css(c: u32) -> String {
    format!("#{c:06X}")
}

/// `a` with `t` of it kept and the rest of `b` (t = 1 gives a).
fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |x: u32, s: u32| ((x >> s) & 0xFF) as f32;
    let m = |s: u32| ((ch(a, s) * t + ch(b, s) * (1.0 - t)).round() as u32) << s;
    m(16) | m(8) | m(0)
}

fn luminance(c: u32) -> f32 {
    let f = |s: u32| {
        let v = ((c >> s) & 0xFF) as f32 / 255.0;
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * f(16) + 0.7152 * f(8) + 0.0722 * f(0)
}

/// Removes an added library item again.
pub fn remove(id: &str) -> Value {
    let mut have = installed();
    let Some(entry) = have.remove(id) else { return json!({ "ok": false }) };
    let undo = &entry["undo"];
    if let (Some(kind), Some(sid)) = (undo["sideload"].as_str(), undo["id"].as_str()) {
        crate::sideload::remove(kind, sid);
    }
    if let (Some(list), Some(keys)) = (undo["list"].as_str(), undo["keys"].as_array()) {
        let list = match list { "wordlist" => List::Words, "autocorrect" => List::Autocorrect, _ => List::Shortcuts };
        if let Ok(db) = UserData::open(&likhon_store::user_db(&crate::settings::dir().0)) {
            keys.iter().filter_map(Value::as_str).for_each(|k| {
                db.remove(list, k);
            });
        }
    }
    if let Some(files) = undo["fonts"].as_array() {
        files.iter().filter_map(Value::as_str).for_each(fonts::uninstall);
    }
    save_installed(&have);
    crate::settings_app::notify_agent(crate::settings_app::MSG_USER_CHANGED, 0);
    crate::settings_app::notify_agent(crate::settings_app::MSG_SETTINGS_CHANGED, 0);
    json!({ "ok": true })
}

// ---- Dictionary ------------------------------------------------------------------------------------------------

/// Downloads the dictionary into the dict folder beside likhon.exe (for a Portable copy without one, or a newer one).
pub fn dictionary() -> Value {
    let run = || -> Result<Value, String> {
        let info = json_from(&format!("{}/api/app/dictionary", net::site()))?;
        let url = info["url"].as_str().ok_or("no dictionary is published yet")?;
        let body = net::get(url)?;
        if net::sha256_hex(&body) != info["sha256"].as_str().unwrap_or("").to_ascii_lowercase() {
            return Err("the download does not match (checksum)".into());
        }
        let dir = std::env::current_exe().map_err(|e| e.to_string())?.parent().ok_or("no folder")?.join("dict");
        std::fs::create_dir_all(&dir).map_err(|_| "the folder beside likhon.exe cannot be written to")?;
        let tmp = dir.join("words.sqlite.download");
        std::fs::write(&tmp, &body).map_err(|_| "the folder beside likhon.exe cannot be written to")?;
        // Only a real Likhon dictionary replaces the old one.
        let words = likhon_store::Dictionary::open(&tmp).map(|d| d.info().words).map_err(|_| {
            let _ = std::fs::remove_file(&tmp);
            "the download is not a Likhon dictionary".to_string()
        })?;
        let target = dir.join("words.sqlite");
        let _ = std::fs::remove_file(&target);
        std::fs::rename(&tmp, &target).map_err(|_| "the dictionary is in use; close Likhon and try again")?;
        crate::settings_app::notify_agent(crate::settings_app::MSG_USER_CHANGED, 0);
        Ok(json!({ "ok": true, "words": words, "version": info["version"] }))
    };
    run().unwrap_or_else(|e| json!({ "ok": false, "error": e }))
}

// ---- Donating words --------------------------------------------------------------------------------------------

/// Sends the words the person picked (Privacy › Donate words): words only, nothing about who sent them.
pub fn donate(words: &[String]) -> Value {
    let clean: Vec<&String> = words.iter().filter(|w| !w.is_empty() && w.chars().count() <= 40 && w.chars().all(|c| ('\u{0980}'..='\u{09FF}').contains(&c) || c == '\u{200C}' || c == '\u{200D}')).take(500).collect();
    if clean.is_empty() {
        return json!({ "ok": false, "error": "no Bangla words to send" });
    }
    match net::post_json(&format!("{}/api/dictionary/donate", net::site()), &json!({ "words": clean }).to_string()) {
        Ok(b) => json!({ "ok": true, "accepted": serde_json::from_slice::<Value>(&b).ok().map(|v| v["accepted"].clone()).unwrap_or(Value::Null) }),
        Err(e) => json!({ "ok": false, "error": e }),
    }
}

/// Fonts from the library, installed for this Windows user only (no administrator; Windows 10 1809 or later).
pub mod fonts {
    use windows_sys::Win32::Foundation::{LPARAM, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{AddFontResourceW, RemoveFontResourceW};
    use windows_sys::Win32::System::Registry::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, HWND_BROADCAST, WM_FONTCHANGE};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn folder() -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("Microsoft").join("Windows").join("Fonts")
    }

    const KEY: &str = "Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts";

    /// Installs the .ttf and .otf files of a library font zip. Returns the installed paths (for removing them).
    pub fn install_zip(zip: &[u8]) -> Result<Vec<String>, String> {
        let entries = super::zip::read(zip)?;
        let dir = folder();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut done = Vec::new();
        for (name, bytes) in entries {
            let lower = name.to_ascii_lowercase();
            let file = name.rsplit(['/', '\\']).next().unwrap_or(&name).to_string();
            if !(lower.ends_with(".ttf") || lower.ends_with(".otf")) || file.is_empty() || file.contains("..") {
                continue;
            }
            if !matches!(&bytes.get(..4), Some(b"\x00\x01\x00\x00" | b"true" | b"OTTO")) {
                continue; // not a font: skipped, never installed
            }
            let path = dir.join(&file);
            std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
            let p = path.to_string_lossy().to_string();
            let stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(&file);
            let kind = if lower.ends_with(".otf") { "OpenType" } else { "TrueType" };
            unsafe {
                let mut key = std::ptr::null_mut();
                if RegCreateKeyExW(HKEY_CURRENT_USER, wide(KEY).as_ptr(), 0, std::ptr::null(), 0, KEY_SET_VALUE, std::ptr::null(), &mut key, std::ptr::null_mut()) == 0 {
                    let v = wide(&p);
                    RegSetValueExW(key, wide(&format!("{stem} ({kind})")).as_ptr(), 0, REG_SZ, v.as_ptr() as *const u8, (v.len() * 2) as u32);
                    RegCloseKey(key);
                }
                AddFontResourceW(wide(&p).as_ptr());
            }
            done.push(p);
        }
        if done.is_empty() {
            return Err("the zip has no font files".into());
        }
        unsafe { PostMessageW(HWND_BROADCAST, WM_FONTCHANGE, 0 as WPARAM, 0 as LPARAM) };
        Ok(done)
    }

    /// Removes one font this page installed (its file, its registry entry, and from the running session).
    pub fn uninstall(path: &str) {
        let p = std::path::Path::new(path);
        // Only files in this user's font folder, which is where install_zip puts them.
        if p.parent() != Some(folder().as_path()) {
            return;
        }
        unsafe {
            RemoveFontResourceW(wide(path).as_ptr());
            let mut key = std::ptr::null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, wide(KEY).as_ptr(), 0, KEY_SET_VALUE, &mut key) == 0 {
                let file = p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
                let stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(&file).to_string();
                for kind in ["TrueType", "OpenType"] {
                    RegDeleteValueW(key, wide(&format!("{stem} ({kind})")).as_ptr());
                }
                RegCloseKey(key);
            }
            PostMessageW(HWND_BROADCAST, WM_FONTCHANGE, 0, 0);
        }
        let _ = std::fs::remove_file(p);
    }
}

/// A small zip reader for library fonts: stored and deflated entries, sizes capped.
pub mod zip {
    const MAX_ENTRY: usize = 32 * 1024 * 1024;

    fn u16le(b: &[u8], at: usize) -> Option<usize> {
        Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?) as usize)
    }
    fn u32le(b: &[u8], at: usize) -> Option<usize> {
        Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as usize)
    }

    /// Every file in the zip as (name, bytes), read from the central directory.
    pub fn read(z: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
        let bad = || "not a valid zip".to_string();
        // End of central directory: the last "PK\x05\x06" in the file.
        let eocd = (0..z.len().saturating_sub(21)).rev().find(|&i| z[i..].starts_with(b"PK\x05\x06")).ok_or_else(bad)?;
        let count = u16le(z, eocd + 10).ok_or_else(bad)?;
        let mut at = u32le(z, eocd + 16).ok_or_else(bad)?;
        let mut out = Vec::new();
        for _ in 0..count.min(512) {
            if !z.get(at..).is_some_and(|r| r.starts_with(b"PK\x01\x02")) {
                return Err(bad());
            }
            let method = u16le(z, at + 10).ok_or_else(bad)?;
            let csize = u32le(z, at + 20).ok_or_else(bad)?;
            let usize_ = u32le(z, at + 24).ok_or_else(bad)?;
            let nlen = u16le(z, at + 28).ok_or_else(bad)?;
            let xlen = u16le(z, at + 30).ok_or_else(bad)?;
            let clen = u16le(z, at + 32).ok_or_else(bad)?;
            let local = u32le(z, at + 42).ok_or_else(bad)?;
            let name = String::from_utf8_lossy(z.get(at + 46..at + 46 + nlen).ok_or_else(bad)?).to_string();
            at += 46 + nlen + xlen + clen;
            if usize_ > MAX_ENTRY || name.ends_with('/') {
                continue;
            }
            let lnlen = u16le(z, local + 26).ok_or_else(bad)?;
            let lxlen = u16le(z, local + 28).ok_or_else(bad)?;
            let start = local + 30 + lnlen + lxlen;
            let raw = z.get(start..start + csize).ok_or_else(bad)?;
            let bytes = match method {
                0 => raw.to_vec(),
                8 => miniz_oxide::inflate::decompress_to_vec_with_limit(raw, MAX_ENTRY).map_err(|_| bad())?,
                _ => continue,
            };
            out.push((name, bytes));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_websites_layouts_and_themes_become_app_files() {
        let theme = r##"{"format":"likhon-theme","name":{"en":"Shapla","bn":"শাপলা"},"license":"MIT","version":"1.0","best":"light",
            "tokens":{"light":{"background":"#FFF8F5","border":"#EBCFD9","accent":"#B3305F","text":"#2A1A20","selected-row-bg":"#FBE3EC"},
                      "dark":{"background":"#2A1E23","border":"#4A3540","accent":"#F28DB2","text":"#F6E9EE","selected-row-bg":"#4A2636"}}}"##;
        let t = library_format("theme", "shapla", theme).unwrap();
        assert_eq!(crate::sideload::check_theme(&t), Ok(("shapla".to_string(), "Shapla".to_string())), "{t}");
        let layout = r#"{"format":"likhon-layout","name":{"en":"Mine"},"license":"MIT","version":"1.2","keys":{"k":{"normal":"ক"}}}"#;
        let l = library_format("layout", "sample-layout", layout).unwrap();
        let parsed = likhon_engine::fixed::Layout::from_json(&l).unwrap();
        assert_eq!(parsed.id, "sample-layout");
        // A file already in the app's shape is left as it is.
        assert_eq!(library_format("layout", "x", &l).unwrap(), l);
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("1.0.1", "1.0.0"));
        assert!(!newer("1.0.0", "1.0.0"));
        assert!(!newer("0.2", "0.10"));
    }

    /// Talks to the live site and needs the network (build with LIKHON_SITE=https://… for another site): cargo test -p likhon -- --ignored online
    #[test]
    #[ignore]
    fn online_against_the_site() {
        let l = latest();
        assert_eq!(l["ok"], true, "{l}");
        let lib = library();
        assert_eq!(lib["ok"], true, "{lib}");
        let items = lib["items"].as_array().unwrap();
        assert!(!items.is_empty());
        let font = items.iter().find(|i| i["type"] == "font").expect("a font");
        let body = net::get(font["file"].as_str().unwrap()).unwrap();
        assert_eq!(net::sha256_hex(&body), font["sha256"].as_str().unwrap());
        assert!(zip::read(&body).unwrap().iter().any(|(n, _)| n.to_lowercase().ends_with(".ttf") || n.to_lowercase().ends_with(".otf")));
        let d = json_from(&format!("{}/api/app/dictionary", net::site())).unwrap();
        assert!(d["url"].as_str().is_some_and(net::on_site), "{d}");
        assert!(!net::on_site("https://example.com/x") && !net::on_site("http://likhon.test/"));
        // Every layout and theme the site lists can be added (its own format is translated).
        for it in items.iter().filter(|i| matches!(i["type"].as_str(), Some("layout" | "theme"))) {
            let body = net::get(it["file"].as_str().unwrap()).unwrap();
            let kind = it["type"].as_str().unwrap();
            if it["file_name"].as_str().is_some_and(|f| f.ends_with(".mim")) {
                let im = likhon_engine::mit::khipro::InputMethod::parse(&String::from_utf8(body).unwrap());
                assert!(im.takes('a') && !im.convert("ami").is_empty(), "{}", it["id"]);
                continue;
            }
            let text = library_format(kind, it["id"].as_str().unwrap(), &String::from_utf8(body).unwrap()).unwrap();
            let ok = if kind == "layout" { likhon_engine::fixed::Layout::from_json(&text).map(|_| ()) } else { crate::sideload::check_theme(&text).map(|_| ()) };
            assert!(ok.is_ok(), "{}: {:?}", it["id"], ok);
        }
        println!("latest {l}
items {}", items.len());
    }
}
