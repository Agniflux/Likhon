// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! Learning from the Likhon keyboard (likhon_tsf.dll). The keyboard shows its own suggestions (tsf/src/suggest.rs);
//! when a word is finished or picked it sends COMMIT "word" by WM_COPYDATA to the agent's hidden window, and the agent
//! learns it with the same rules as for the hook: learning on, Bangla, and not in a password field (UI Automation).
//! Nothing typed is logged or kept here otherwise.
//!
//! The keyboard waits while its message is handled, so WM_COPYDATA only queues the word and returns: the password
//! check asks the app, which would still be waiting (it stalled both in Word, 2026-10-24); it runs just after
//! (MSG_BRIDGE). The tag is the same number in tsf/src/suggest.rs.

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Posted to the agent window by copydata: handle the queued requests (agent.rs calls `process`).
pub const MSG_BRIDGE: u32 = WM_APP + 31;
const TAG_COMMIT: usize = 0x4C4B_0004;

/// Lets the keyboard reach the agent from apps that run with less permission (Store apps).
pub fn allow(hwnd: HWND) {
    unsafe { ChangeWindowMessageFilterEx(hwnd, WM_COPYDATA, MSGFLT_ALLOW, std::ptr::null_mut()) };
}

fn text(cds: &COPYDATASTRUCT) -> Option<String> {
    if cds.lpData.is_null() || cds.cbData % 2 != 0 || cds.cbData > 4096 {
        return None;
    }
    let w = unsafe { std::slice::from_raw_parts(cds.lpData as *const u16, cds.cbData as usize / 2) };
    Some(String::from_utf16_lossy(w))
}

/// Requests waiting for MSG_BRIDGE: (keyboard window, tag, body). Only the agent thread touches it.
static QUEUE: std::sync::Mutex<Vec<(usize, usize, String)>> = std::sync::Mutex::new(Vec::new());

/// WM_COPYDATA on the agent's window: queue it and return at once.
pub fn copydata(agent: HWND, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let keyboard = wparam as HWND;
    let Some(cds) = (unsafe { (lparam as *const COPYDATASTRUCT).as_ref() }) else { return 0 };
    if unsafe { IsWindow(keyboard) } == 0 || cds.dwData != TAG_COMMIT {
        return 0;
    }
    let body = text(cds).unwrap_or_default();
    if let Ok(mut q) = QUEUE.lock() {
        if q.len() < 64 {
            q.push((keyboard as usize, cds.dwData, body));
        }
    }
    unsafe { PostMessageW(agent, MSG_BRIDGE, 0, 0) };
    1
}

/// MSG_BRIDGE: handle the queued requests.
pub fn process(agent: HWND) {
    let queued = QUEUE.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
    for (keyboard, tag, body) in queued {
        handle(agent, keyboard as HWND, tag, body);
    }
}

fn handle(_agent: HWND, keyboard: HWND, tag: usize, body: String) {
    if unsafe { IsWindow(keyboard) } == 0 || tag != TAG_COMMIT {
        return;
    }
    let s = crate::agent::settings();
    let bangla = body.chars().any(|c| ('\u{0980}'..='\u{09FF}').contains(&c));
    if s.learn && bangla && body.chars().count() <= 64 && crate::suggest::password_field() == Some(false) {
        crate::suggest::learn(&[(None, body)]);
    }
}
