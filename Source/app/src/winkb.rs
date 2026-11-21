//! Likhon in Windows' keyboard list (Win+Space) from any copy, Portable included (owner decision 2026-10-24).
//!
//! Windows reads text-service profiles only from HKLM (SPIKES.md M3: per-user registration was tried and refused), so
//! adding needs administrator permission once. Settings starts `likhon.exe --keyboard-add <data folder>` elevated:
//! it writes the two text-service DLLs embedded in this exe (64-bit and 32-bit, built from tsf/) to
//! Program Files\Likhon and Program Files (x86)\Likhon, the same places scripts/register-tsf.ps1 and the installer
//! use, with datadir.txt naming this copy's data folder (and, line 2, its dictionary), and registers both with regsvr32. `--keyboard-remove`
//! unregisters them and deletes the files; a DLL still loaded by a running app is renamed and deleted at restart.
//! The keyboard is added to or taken out of this user's list by Settings itself (not elevated): user_list().
//! Every step is logged in %ProgramData%\Likhon\tsf.log.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};

/// The tip string Windows uses for the Likhon profile: Bangla (Bangladesh), CLSID_LIKHON, PROFILE_LIKHON (tsf/src/lib.rs).
const TIP: &str = "0845:{C772DC88-B930-41B1-A45A-C800A07FAD7C}{1A047B60-A777-42B7-B08B-2329083DCFE3}";
const DLL: &str = "likhon_tsf.dll";

/// The DLLs, deflated by build.rs (empty when the build had no DLLs: see build.rs).
static TSF64: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tsf64.dll.z"));
static TSF86: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tsf86.dll.z"));

/// Exit codes of the elevated helper, read by Settings.
pub const NO_FILES: i32 = 3;
const WRITE_FAILED: i32 = 4;
const REGISTER_FAILED: i32 = 5;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn env_dir(name: &str, fallback: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| fallback.into()))
}

/// (folder, DLL bytes, regsvr32 for that bitness).
fn targets() -> [(PathBuf, &'static [u8], PathBuf); 2] {
    let win = env_dir("WINDIR", r"C:\Windows");
    [
        (env_dir("ProgramFiles", r"C:\Program Files").join("Likhon"), TSF64, win.join(r"System32\regsvr32.exe")),
        (env_dir("ProgramFiles(x86)", r"C:\Program Files (x86)").join("Likhon"), TSF86, win.join(r"SysWOW64\regsvr32.exe")),
    ]
}

fn log(line: &str) {
    use std::io::Write;
    let dir = env_dir("ProgramData", r"C:\ProgramData").join("Likhon");
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("tsf.log")) {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "unix:{t} likhon.exe: {line}");
    }
}

fn regsvr32(exe: &Path, dll: &Path, undo: bool) -> bool {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut c = std::process::Command::new(exe);
    c.arg("/s");
    if undo {
        c.arg("/u");
    }
    let ok = c.arg(dll).creation_flags(CREATE_NO_WINDOW).status().map(|s| s.success()).unwrap_or(false);
    log(&format!("regsvr32{} {}: {}", if undo { " /u" } else { "" }, dll.display(), if ok { "ok" } else { "failed" }));
    ok
}

/// Moves a DLL some app still has loaded out of the way (renaming a loaded DLL is allowed) and deletes it at restart.
fn set_aside(file: &Path) -> bool {
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT};
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let old = file.with_extension(format!("dll.{t}.old"));
    if std::fs::rename(file, &old).is_err() {
        return false;
    }
    unsafe { MoveFileExW(wide(&old.to_string_lossy()).as_ptr(), std::ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT) };
    log(&format!("in use, deleted at restart: {}", old.display()));
    true
}

/// `likhon.exe --keyboard-add <data folder>` (elevated).
pub fn add(data: &str) -> i32 {
    if TSF64.is_empty() || TSF86.is_empty() {
        log("add: this build has no text-service DLLs");
        return NO_FILES;
    }
    for (dir, packed, _) in targets() {
        let Ok(bytes) = miniz_oxide::inflate::decompress_to_vec(packed) else { return NO_FILES };
        let file = dir.join(DLL);
        if std::fs::read(&file).is_ok_and(|b| b == bytes) {
            log(&format!("add: {} is current", file.display()));
        } else {
            let _ = std::fs::create_dir_all(&dir);
            if std::fs::write(&file, &bytes).is_err() && !(set_aside(&file) && std::fs::write(&file, &bytes).is_ok()) {
                log(&format!("add: cannot write {}", file.display()));
                return WRITE_FAILED;
            }
            log(&format!("add: wrote {}", file.display()));
        }
        // Line 2: the dictionary beside this likhon.exe, for the keyboard's own suggestion window (tsf/src/suggest.rs).
        let dict = crate::settings::dictionary_path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        if std::fs::write(dir.join("datadir.txt"), format!("{data}\n{dict}")).is_err() {
            return WRITE_FAILED;
        }
    }
    log(&format!("add: settings from {data}"));
    for (dir, _, reg) in targets() {
        if !regsvr32(&reg, &dir.join(DLL), false) {
            return REGISTER_FAILED;
        }
    }
    0
}

/// Likhon is in the keyboard list, but this copy carries a different keyboard than the one installed: Settings offers
/// Update (the same elevated `--keyboard-add`, which rewrites the files).
pub fn stale() -> bool {
    targets().into_iter().any(|(dir, packed, _)| {
        let Ok(installed) = std::fs::read(dir.join(DLL)) else { return false };
        !packed.is_empty() && miniz_oxide::inflate::decompress_to_vec(packed).is_ok_and(|ours| ours != installed)
    })
}

/// `likhon.exe --keyboard-remove` (elevated). Leaves anything else in the folders (an installed likhon.exe) alone.
pub fn remove() -> i32 {
    for (dir, _, reg) in targets() {
        let file = dir.join(DLL);
        if file.exists() {
            regsvr32(&reg, &file, true);
            if std::fs::remove_file(&file).is_ok() {
                log(&format!("remove: deleted {}", file.display()));
            } else {
                set_aside(&file);
            }
        }
        let _ = std::fs::remove_file(dir.join("datadir.txt"));
        let _ = std::fs::remove_dir(&dir); // only when empty
    }
    0
}

/// Adds the Likhon keyboard to this Windows user's list (Bangla (Bangladesh), added if missing) or takes it out,
/// through input.dll's InstallLayoutOrTip. Runs in Settings, not elevated: the list belongs to the user.
pub fn user_list(add: bool) -> bool {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    const ILOT_UNINSTALL: u32 = 1;
    type InstallLayoutOrTip = unsafe extern "system" fn(*const u16, u32) -> i32;
    unsafe {
        let lib = LoadLibraryW(wide("input.dll").as_ptr());
        if lib.is_null() {
            return false;
        }
        let Some(f) = GetProcAddress(lib, c"InstallLayoutOrTip".as_ptr().cast()) else { return false };
        let f: InstallLayoutOrTip = std::mem::transmute(f);
        f(wide(TIP).as_ptr(), if add { 0 } else { ILOT_UNINSTALL }) != 0
    }
}
