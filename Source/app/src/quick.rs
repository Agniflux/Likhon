// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! Quick hotkeys on the selected text (Settings › Hotkeys; v2 §11 quick popups): Ctrl+Alt+U converts the selection
//! between Unicode and Bijoy, Ctrl+Alt+Z undoes that conversion, Ctrl+Alt+W adds the selected Bangla words to My
//! words. Ctrl+Alt+T, H and S open the Tools page, How do I type this and the symbol palette in Settings.
//!
//! The selection is read through the clipboard (Ctrl+C), the only way that works in every app. What the clipboard
//! held as text is put back a moment later; other clipboard content (a picture) is not kept. Unverified across apps
//! (Desktop/CLAUDE.md 5, legacy encodings): see tests/compat. Left Alt only, so AltGr keeps typing layout levels.
//! Nothing here is logged or stored except the words the person asked to add.

use crate::settings_app::win::{clipboard_text, set_clipboard_text};
use windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

/// Hotkey letters (with Ctrl + left Alt) and what they do.
pub const KEYS: [u16; 6] = [b'U' as u16, b'Z' as u16, b'W' as u16, b'T' as u16, b'H' as u16, b'S' as u16];

/// Marks our own injected keys, so the hook lets them through (the same marker agent.rs uses).
const MAGIC: usize = crate::agent::MAGIC;

unsafe fn keys(seq: &[(u16, bool)]) {
    let inputs: Vec<INPUT> = seq
        .iter()
        .map(|&(vk, up)| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: MAGIC } },
        })
        .collect();
    SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
}

/// Ctrl + `vk` in the focused app, with Alt released first (the person still holds Ctrl+Alt from the hotkey). An
/// unassigned key goes between, so releasing Alt does not open the app's menu.
unsafe fn ctrl(vk: u16) {
    keys(&[(0xE8, false), (0xE8, true), (VK_MENU, true), (VK_LMENU, true), (VK_CONTROL, false), (vk, false), (vk, true)]);
    if GetAsyncKeyState(VK_CONTROL as i32) >= 0 {
        keys(&[(VK_CONTROL, true)]);
    }
}

/// The selected text in the focused app, and what the clipboard held before, as text.
fn copy_selection() -> Option<(String, Option<String>)> {
    let before = clipboard_text();
    unsafe {
        let seq = GetClipboardSequenceNumber();
        ctrl(b'C' as u16);
        for _ in 0..25 {
            std::thread::sleep(std::time::Duration::from_millis(20));
            if GetClipboardSequenceNumber() != seq {
                // Some apps write the clipboard in steps: give them a moment more.
                std::thread::sleep(std::time::Duration::from_millis(30));
                return clipboard_text().filter(|s| !s.is_empty()).map(|s| (s, before));
            }
        }
    }
    None
}

fn restore_later(before: Option<String>) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(b) = before {
            set_clipboard_text(&b);
        }
    });
}

thread_local! {
    /// When the last conversion was pasted: Ctrl+Alt+Z undoes it within a minute.
    static LAST: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) };
}

/// Runs a quick hotkey on the agent thread (not in the keyboard hook).
pub fn run(vk: u16) {
    match vk as u8 {
        b'U' => convert(),
        b'Z' => {
            if LAST.with(|l| l.get()).is_some_and(|t| t.elapsed().as_secs() < 60) {
                LAST.with(|l| l.set(None));
                unsafe { ctrl(b'Z' as u16) };
            }
        }
        b'W' => add_words(),
        b'T' => crate::agent::open_settings(Some("tools")),
        b'H' => crate::agent::open_settings(Some("tools:how")),
        b'S' => crate::agent::open_settings(Some("tools:sym")),
        _ => {}
    }
}

fn is_bangla(c: char) -> bool {
    ('\u{0980}'..='\u{09FF}').contains(&c)
}

/// Unicode Bangla becomes Bijoy; anything else is read as Bijoy text and becomes Unicode.
fn convert() {
    let Some((text, before)) = copy_selection() else {
        crate::agent::notice("app.quick.selectH", "app.quick.selectP", &[("key", "Ctrl + Alt + U")]);
        return;
    };
    if text.chars().count() > 20_000 {
        restore_later(before);
        return;
    }
    let out = if text.chars().any(is_bangla) { likhon_engine::unicode_to_bijoy(&text) } else { likhon_engine::bijoy_to_unicode(&text) };
    set_clipboard_text(&out);
    unsafe { ctrl(b'V' as u16) };
    LAST.with(|l| l.set(Some(std::time::Instant::now())));
    restore_later(before);
}

/// The selected Bangla words go to My words (user.sqlite), and Likhon says which.
fn add_words() {
    let Some((text, before)) = copy_selection() else {
        crate::agent::notice("app.quick.selectH", "app.quick.selectP", &[("key", "Ctrl + Alt + W")]);
        return;
    };
    restore_later(before);
    let words: Vec<String> = text
        .split(|c: char| !(is_bangla(c) || c == '\u{200C}' || c == '\u{200D}'))
        .filter(|w| w.chars().count() >= 2 && !w.chars().all(|c| ('\u{09E6}'..='\u{09EF}').contains(&c)))
        .take(50)
        .map(String::from)
        .collect();
    let Ok(db) = likhon_store::UserData::open(&likhon_store::user_db(&crate::settings::dir().0)) else { return };
    let added: Vec<String> = words.into_iter().filter(|w| db.add(likhon_store::List::Words, w, "").is_ok()).collect();
    if added.is_empty() {
        crate::agent::notice("app.quick.selectH", "app.quick.selectP", &[("key", "Ctrl + Alt + W")]);
        return;
    }
    crate::settings_app::notify_agent(crate::settings_app::MSG_USER_CHANGED, 0);
    crate::agent::notice("app.quick.addedH", "app.quick.addedP", &[("words", &added.join(", "))]);
}
