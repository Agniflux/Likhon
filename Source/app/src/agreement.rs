// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! The first-run agreement (Desktop/CLAUDE.md section 9; the final legal documents of 2026-11-21).
//!
//! Portable has no installer licence page, so the first start shows the software license agreement in the Settings
//! window (`likhon.exe --settings agreement`, a separate process) with Accept and Decline. Decline, or closing the
//! window, exits without typing anything. The accepted date is kept in settings.json and asked again when the
//! agreement's date changes. If the window cannot open (no WebView2), a plain native question is asked instead.

use crate::settings;
use crate::text::{t, wide};
use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDYES, MB_ICONINFORMATION, MB_YESNO};

/// Exit code of the Settings process when its window could not open.
pub const NO_WINDOW: i32 = 3;

/// True when the agent may start: the current agreement is accepted (now or earlier).
pub fn gate() -> bool {
    if settings::agreement_accepted() {
        return true;
    }
    let code = std::env::current_exe().ok().and_then(|exe| std::process::Command::new(exe).args(["--settings", "agreement"]).status().ok()).and_then(|s| s.code());
    if code == Some(NO_WINDOW) {
        fallback();
    }
    settings::agreement_accepted()
}

/// A native question, for a PC where the Settings window cannot open. Short, with the address of the full text.
fn fallback() {
    let lang = settings::load().language;
    let body = t("app.set.agreement.fallback", lang, &[("site", &settings::site()), ("date", settings::AGREEMENT_DATE)]);
    let yes = unsafe { MessageBoxW(std::ptr::null_mut(), wide(&body).as_ptr(), wide(&t("app.set.agreement.title", lang, &[])).as_ptr(), MB_YESNO | MB_ICONINFORMATION) };
    if yes == IDYES {
        settings::set("agreement_accepted", settings::AGREEMENT_DATE.into());
    }
}
