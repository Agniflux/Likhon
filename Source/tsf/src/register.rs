// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! Registration (regsvr32 likhon_tsf.dll, as administrator): the COM class under HKLM\Software\Classes, the keyboard
//! profile for Bangla (Bangladesh), and the categories that make it a keyboard usable in Store apps and the tray's
//! language switcher. Unregistration removes all three. Scripts: scripts/register-tsf.ps1, scripts/unregister-tsf.ps1.
//!
//! Whether a profile can be registered per user without administrator permission is the open M3 question; this
//! registers machine-wide, the way Windows' own samples do.

use super::{CLSID_LIKHON, LANGID_BN_BD, PROFILE_LIKHON};
use std::path::PathBuf;
use windows::Win32::Foundation::{E_FAIL, HMODULE};
use windows::Win32::System::Com::*;
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT};
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::*;
use windows_core::{GUID, PCWSTR};

/// This DLL's own path.
pub fn module_path() -> Option<PathBuf> {
    unsafe {
        let mut h = HMODULE::default();
        GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, PCWSTR(module_path as *const u16), &mut h).ok()?;
        let mut buf = vec![0u16; 1024];
        let n = GetModuleFileNameW(Some(h), &mut buf) as usize;
        (n > 0).then(|| PathBuf::from(String::from_utf16_lossy(&buf[..n])))
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn guid_text(g: &GUID) -> String {
    format!("{{{g:?}}}")
}

fn set_sz(key: HKEY, name: Option<&str>, value: &str) -> windows_core::Result<()> {
    let v = wide(value);
    let n = name.map(wide);
    let bytes = unsafe { core::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2) };
    unsafe { RegSetValueExW(key, n.as_ref().map(|n| PCWSTR(n.as_ptr())).unwrap_or(PCWSTR::null()), None, REG_SZ, Some(bytes)).ok() }
}

fn create(parent: HKEY, path: &str) -> windows_core::Result<HKEY> {
    let mut k = HKEY::default();
    unsafe { RegCreateKeyExW(parent, PCWSTR(wide(path).as_ptr()), None, PCWSTR::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut k, None).ok()? };
    Ok(k)
}

const CATEGORIES: [GUID; 4] = [GUID_TFCAT_TIP_KEYBOARD, GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT, GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, GUID_TFCAT_TIPCAP_UIELEMENTENABLED];

pub fn register() -> windows_core::Result<()> {
    let dll = module_path().ok_or_else(|| windows_core::Error::from(E_FAIL))?;
    let dll_text = dll.to_string_lossy().into_owned();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        // The COM class.
        let clsid = format!(r"Software\Classes\CLSID\{}", guid_text(&CLSID_LIKHON));
        let k = create(HKEY_LOCAL_MACHINE, &clsid)?;
        set_sz(k, None, "Likhon")?;
        let _ = RegCloseKey(k);
        let k = create(HKEY_LOCAL_MACHINE, &format!(r"{clsid}\InprocServer32"))?;
        set_sz(k, None, &dll_text)?;
        set_sz(k, Some("ThreadingModel"), "Apartment")?;
        let _ = RegCloseKey(k);
        // The keyboard profile.
        let profiles: ITfInputProcessorProfileMgr = CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        let desc: Vec<u16> = "লিখন (Likhon)".encode_utf16().collect();
        let icon: Vec<u16> = dll_text.encode_utf16().collect();
        profiles.RegisterProfile(&CLSID_LIKHON, LANGID_BN_BD, &PROFILE_LIKHON, &desc, &icon, 0, HKL::default(), 0, true, 0)?;
        let cats: ITfCategoryMgr = CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
        for c in CATEGORIES {
            cats.RegisterCategory(&CLSID_LIKHON, &c, &CLSID_LIKHON)?;
        }
    }
    Ok(())
}

pub fn unregister() -> windows_core::Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if let Ok(cats) = CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER) {
            for c in CATEGORIES {
                let _ = cats.UnregisterCategory(&CLSID_LIKHON, &c, &CLSID_LIKHON);
            }
        }
        if let Ok(p) = CoCreateInstance::<_, ITfInputProcessorProfileMgr>(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER) {
            let _ = p.UnregisterProfile(&CLSID_LIKHON, LANGID_BN_BD, &PROFILE_LIKHON, 0);
        }
        let clsid = format!(r"Software\Classes\CLSID\{}", guid_text(&CLSID_LIKHON));
        let _ = RegDeleteTreeW(HKEY_LOCAL_MACHINE, PCWSTR(wide(&clsid).as_ptr()));
    }
    Ok(())
}
