// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! The Default Bangla font changer (Settings › Fonts; Desktop/CLAUDE.md 9). Runs only as `likhon.exe --font-default
//! <face>` or `--font-default-restore`, which Settings starts elevated (Windows asks for administrator permission).
//!
//! What it changes: Windows' font linking (HKLM\…\FontLink\SystemLink), the list of fonts GDI falls back to for
//! letters a base font (Segoe UI, Tahoma, Microsoft Sans Serif, Arial) does not have. The chosen Bangla font goes
//! first in each existing list, so programs that draw with those fonts show Bangla in it. Programs that pick their own
//! font (browsers, Word, most modern apps) are not affected; Windows reads the lists at sign-in.
//!
//! Safety (root CLAUDE.md, elevation): before the first change the lists are saved to
//! %ProgramData%\Likhon\font-default-backup.json, and every change is logged in font-default.log there. Restore puts
//! the saved lists back exactly; scripts/restore-default-font.ps1 does the same by hand. A font installed only for
//! one Windows user is copied into the Windows fonts folder first (font linking reads that folder), and Restore
//! removes the copy. Unverified on this PC until tried (Desktop/CLAUDE.md 9: test on a VM first).

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::*;

const LINK: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\FontLink\SystemLink";
const FONTS: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts";
/// Base fonts whose link lists get the Bangla font: every "Segoe UI…" list and these.
const BASES: [&str; 3] = ["Tahoma", "Microsoft Sans Serif", "Arial"];

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn folder() -> PathBuf {
    PathBuf::from(std::env::var_os("ProgramData").unwrap_or_else(|| "C:\\ProgramData".into())).join("Likhon")
}

fn log(line: &str) {
    use std::io::Write;
    let _ = std::fs::create_dir_all(folder());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(folder().join("font-default.log")) {
        let _ = writeln!(f, "{} {line}", now());
    }
}

fn now() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("unix:{t}")
}

// ---- Registry ----------------------------------------------------------------------------------------------------

struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}

fn open(root: HKEY, path: &str, write: bool) -> Option<Key> {
    let mut k = std::ptr::null_mut();
    let access = if write { KEY_READ | KEY_SET_VALUE } else { KEY_READ };
    (unsafe { RegOpenKeyExW(root, wide(path).as_ptr(), 0, access, &mut k) } == ERROR_SUCCESS).then_some(Key(k))
}

/// Every value of a key as (name, type, data as UTF-16).
fn values(k: &Key) -> Vec<(String, u32, Vec<u16>)> {
    let mut out = Vec::new();
    for i in 0.. {
        let mut name = vec![0u16; 512];
        let mut nlen = name.len() as u32;
        let mut kind = 0u32;
        let mut data = vec![0u16; 8192];
        let mut dlen = (data.len() * 2) as u32;
        let r = unsafe { RegEnumValueW(k.0, i, name.as_mut_ptr(), &mut nlen, std::ptr::null(), &mut kind, data.as_mut_ptr().cast(), &mut dlen) };
        if r != ERROR_SUCCESS {
            break;
        }
        data.truncate(dlen as usize / 2);
        out.push((String::from_utf16_lossy(&name[..nlen as usize]), kind, data));
    }
    out
}

fn multi(data: &[u16]) -> Vec<String> {
    data.split(|&c| c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect()
}

fn sz(data: &[u16]) -> String {
    String::from_utf16_lossy(data).trim_end_matches('\0').to_string()
}

fn set_multi(k: &Key, name: &str, lines: &[String]) -> bool {
    let mut d: Vec<u16> = Vec::new();
    for l in lines {
        d.extend(l.encode_utf16());
        d.push(0);
    }
    d.push(0);
    unsafe { RegSetValueExW(k.0, wide(name).as_ptr(), 0, REG_MULTI_SZ, d.as_ptr().cast(), (d.len() * 2) as u32) == ERROR_SUCCESS }
}

// ---- The change --------------------------------------------------------------------------------------------------

/// The link lists to change: every existing "Segoe UI…" list and the other bases that exist.
fn targets(k: &Key) -> Vec<(String, Vec<String>)> {
    values(k).into_iter().filter(|(n, t, _)| *t == REG_MULTI_SZ && (n.starts_with("Segoe UI") || BASES.contains(&n.as_str()))).map(|(n, _, d)| (n, multi(&d))).collect()
}

/// The registered file of a font family: (registry value name, file as stored, from HKLM?).
pub fn font_file(face: &str) -> Option<(String, String, bool)> {
    let strip = |n: &str| n.trim_end_matches(" (TrueType)").trim_end_matches(" (OpenType)").to_string();
    for (root, machine) in [(HKEY_LOCAL_MACHINE, true), (HKEY_CURRENT_USER, false)] {
        let Some(k) = open(root, FONTS, false) else { continue };
        let all: Vec<(String, String)> = values(&k).into_iter().filter(|(_, t, _)| *t == REG_SZ || *t == REG_EXPAND_SZ).map(|(n, _, d)| (n, sz(&d))).collect();
        let pick = all.iter().find(|(n, _)| strip(n).eq_ignore_ascii_case(face))
            .or_else(|| all.iter().find(|(n, _)| strip(n).eq_ignore_ascii_case(&format!("{face} Regular"))))
            .or_else(|| all.iter().find(|(n, _)| strip(n).to_lowercase().starts_with(&(face.to_lowercase() + " "))));
        if let Some((n, f)) = pick {
            return Some((n.clone(), f.clone(), machine));
        }
    }
    None
}

/// The link line for a face: "file.ttf,Face" for a font in the Windows fonts folder.
pub fn link_line(file_name: &str, face: &str) -> String {
    format!("{file_name},{face}")
}

/// A list with `line` first and the line Likhon added before (`old`) taken out.
pub fn put_first(list: &[String], line: &str, old: Option<&str>) -> Vec<String> {
    std::iter::once(line.to_string()).chain(list.iter().filter(|l| Some(l.as_str()) != old && l.as_str() != line).cloned()).collect()
}

fn backup_path() -> PathBuf {
    folder().join("font-default-backup.json")
}

fn read_backup() -> Option<Map<String, Value>> {
    std::fs::read_to_string(backup_path()).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()).and_then(|v| v.as_object().cloned())
}

fn windows_fonts() -> PathBuf {
    PathBuf::from(std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into())).join("Fonts")
}

/// `--font-default <face>`: the face goes first in the link lists. Exit code 0 when done.
pub fn make_default(face: &str) -> i32 {
    if face.is_empty() || face.len() > 64 || face.contains([',', '\0']) {
        return 2;
    }
    let Some(link) = open(HKEY_LOCAL_MACHINE, LINK, true) else {
        log("change refused: cannot write the font-link key (not elevated?)");
        return 5;
    };
    let Some((reg_name, file, machine)) = font_file(face) else {
        log(&format!("change refused: no installed font named {face:?}"));
        return 3;
    };
    let mut backup = read_backup().unwrap_or_else(|| {
        // First change: save every list as it is now, before anything is touched.
        let mut m = Map::new();
        m.insert("created".into(), json!(now()));
        m.insert("values".into(), Value::Object(targets(&link).into_iter().map(|(n, l)| (n, json!(l))).collect()));
        m
    });
    if !backup_path().exists() {
        let _ = std::fs::create_dir_all(folder());
        if std::fs::write(backup_path(), serde_json::to_string_pretty(&backup).unwrap_or_default()).is_err() {
            log("change refused: cannot write the backup");
            return 4;
        }
        log("backup saved");
    }
    // A per-user font: copy it into the Windows fonts folder, where font linking looks.
    let file_name = if machine {
        Path::new(&file).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or(file.clone())
    } else {
        let src = PathBuf::from(&file);
        let Some(name) = src.file_name().map(|f| f.to_string_lossy().into_owned()) else { return 3 };
        let dst = windows_fonts().join(&name);
        if !dst.exists() {
            if std::fs::copy(&src, &dst).is_err() {
                log(&format!("change refused: cannot copy {} to the Windows fonts folder", src.display()));
                return 6;
            }
            if let Some(fk) = open(HKEY_LOCAL_MACHINE, FONTS, true) {
                let v = wide(&name);
                unsafe { RegSetValueExW(fk.0, wide(&reg_name).as_ptr(), 0, REG_SZ, v.as_ptr().cast(), (v.len() * 2) as u32) };
            }
            backup.insert("copiedFont".into(), json!({ "file": dst.to_string_lossy(), "regName": reg_name }));
            log(&format!("copied {} into the Windows fonts folder", name));
        }
        name
    };
    let line = link_line(&file_name, face);
    let old = backup.get("line").and_then(Value::as_str).map(String::from);
    let mut changed = 0;
    for (name, list) in targets(&link) {
        if set_multi(&link, &name, &put_first(&list, &line, old.as_deref())) {
            changed += 1;
            log(&format!("{name}: first is now {line}"));
        }
    }
    backup.insert("line".into(), json!(line));
    backup.insert("face".into(), json!(face));
    let _ = std::fs::write(backup_path(), serde_json::to_string_pretty(&backup).unwrap_or_default());
    if changed == 0 { 7 } else { 0 }
}

/// `--font-default-restore`: the saved lists back exactly, the copied font removed, the backup deleted.
pub fn restore() -> i32 {
    let Some(backup) = read_backup() else { return 0 };
    let Some(link) = open(HKEY_LOCAL_MACHINE, LINK, true) else {
        log("restore refused: cannot write the font-link key (not elevated?)");
        return 5;
    };
    if let Some(vals) = backup.get("values").and_then(Value::as_object) {
        for (name, v) in vals {
            match v.as_array() {
                Some(a) => {
                    let lines: Vec<String> = a.iter().filter_map(|x| x.as_str().map(String::from)).collect();
                    set_multi(&link, name, &lines);
                }
                None => unsafe {
                    RegDeleteValueW(link.0, wide(name).as_ptr());
                },
            }
            log(&format!("restore: {name}"));
        }
    }
    if let Some(c) = backup.get("copiedFont") {
        if let (Some(file), Some(reg)) = (c["file"].as_str(), c["regName"].as_str()) {
            if let Some(fk) = open(HKEY_LOCAL_MACHINE, FONTS, true) {
                unsafe { RegDeleteValueW(fk.0, wide(reg).as_ptr()) };
            }
            let _ = std::fs::remove_file(file);
            log(&format!("restore: removed {file}"));
        }
    }
    let _ = std::fs::remove_file(backup_path());
    0
}

/// The face Likhon made the default, if any (read without administrator permission).
pub fn current() -> Option<String> {
    read_backup().and_then(|b| b.get("face").and_then(Value::as_str).map(String::from))
}

/// Settings: runs this exe elevated with the arguments and waits. Ok(exit code), or Err("cancelled") when the
/// person said no to the Windows prompt.
pub fn run_elevated(args: &str) -> Result<u32, String> {
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let (verb, file, params) = (wide("runas"), wide(&exe.to_string_lossy()), wide(args));
    unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = params.as_ptr();
        info.nShow = 0;
        if ShellExecuteExW(&mut info) == 0 {
            return Err("cancelled".into());
        }
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 1u32;
        GetExitCodeProcess(info.hProcess, &mut code);
        windows_sys::Win32::Foundation::CloseHandle(info.hProcess);
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bangla_font_goes_first_and_replaces_the_one_likhon_added_before() {
        let list = vec!["TAHOMA.TTF,Tahoma".to_string(), "MSGOTHIC.TTC,MS UI Gothic".to_string()];
        let once = put_first(&list, &link_line("kalpurush.ttf", "Kalpurush"), None);
        assert_eq!(once, vec!["kalpurush.ttf,Kalpurush", "TAHOMA.TTF,Tahoma", "MSGOTHIC.TTC,MS UI Gothic"]);
        let again = put_first(&once, "NotoSansBengali.ttf,Noto Sans Bengali", Some("kalpurush.ttf,Kalpurush"));
        assert_eq!(again, vec!["NotoSansBengali.ttf,Noto Sans Bengali", "TAHOMA.TTF,Tahoma", "MSGOTHIC.TTC,MS UI Gothic"]);
        assert_eq!(put_first(&again, "NotoSansBengali.ttf,Noto Sans Bengali", None).len(), 3, "never twice");
    }

    #[test]
    fn reads_this_pcs_font_link_lists() {
        // Read-only: the lists exist on Windows 10 and 11, and Segoe UI has one.
        let k = open(HKEY_LOCAL_MACHINE, LINK, false).expect("SystemLink key");
        assert!(targets(&k).iter().any(|(n, l)| n == "Segoe UI" && !l.is_empty()));
        assert!(font_file("Nirmala UI").is_some() || font_file("Vrinda").is_some(), "a Bangla system font is registered");
    }
}
