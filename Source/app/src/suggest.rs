// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! The suggestion window and the caret badge (Reference/design-v2/Likhon Desktop.dc.html §4).
//!
//! Suggestion window: 220 DIPs wide, a header with the typed letters and a pin (follow the cursor, or stay anchored),
//! then rows of 34: number, word, tag. Order (§4 note): the autocorrect match, then plain phonetic, then close
//! dictionary words (the person's own words first). The selected row follows Typing › Selected suggestion. It never
//! takes focus (WS_EX_NOACTIVATE, MA_NOACTIVATE); the keyboard hook moves the selection (Up, Down) and commits it
//! (Space, Enter); Escape hides it. Hidden in password fields that Windows reports (ES_PASSWORD edit controls); other
//! password fields are not detected yet (Desktop/CLAUDE.md 5: UI Automation, unverified).
//!
//! Caret badge: অ or A beside the caret for 700 ms, then a 160 ms fade, when typing switches (§4).
//!
//! The candidates are worked out on the agent's message loop (WM_APP + 20), never inside the keyboard hook.


use crate::settings::Settings;
use crate::text::wide;
use crate::topbar::{self, Gfx, Painter};
// The rows and their order are shared with the Likhon keyboard (likhon_tsf.dll): likhon-store/src/rank.rs.
pub use likhon_store::rank::{candidates, choose, preselect, Candidate, Tag};
use likhon_store::{Dictionary, UserData};
use std::cell::RefCell;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::{BeginPaint, ClientToScreen, EndPaint, GetMonitorInfoW, InvalidateRect, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Posted to the agent window after a key changed the word: recompute and show.
pub const WM_SUGGEST: u32 = WM_APP + 20;

const W: f32 = 220.0;
const HEAD: f32 = 30.0;
const ROW: f32 = 34.0;
const PAD: f32 = 3.0;
const BADGE_MS: u32 = 700;
const FADE_MS: u32 = 160;
const TIMER_BADGE: usize = 1;
const TIMER_FADE: usize = 2;

struct Popup {
    gfx: Gfx,
    hwnd: HWND,
    badge: HWND,
    target: Option<ID2D1HwndRenderTarget>,
    badge_target: Option<ID2D1HwndRenderTarget>,
    keys: String,
    items: Vec<Candidate>,
    selected: usize,
    visible: bool,
    anchored: bool,
    font: f32,
    badge_bangla: bool,
    badge_alpha: u8,
    dict: Option<Dictionary>,
    user: Option<UserData>,
}

thread_local! {
    static POPUP: RefCell<Option<Popup>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut Popup) -> R) -> Option<R> {
    POPUP.with(|p| p.try_borrow_mut().ok()?.as_mut().map(f))
}

pub fn start() {
    let Some(gfx) = Gfx::new() else { return };
    let hwnd = window("LikhonSuggest", false);
    let badge = window("LikhonCaretBadge", true);
    if hwnd.is_null() || badge.is_null() {
        return;
    }
    let dict = crate::settings::dictionary_path().and_then(|p| Dictionary::open(&p).ok());
    let user = UserData::open(&likhon_store::user_db(&crate::settings::dir().0)).ok();
    POPUP.with(|p| {
        *p.borrow_mut() = Some(Popup {
            gfx,
            hwnd,
            badge,
            target: None,
            badge_target: None,
            keys: String::new(),
            items: Vec::new(),
            selected: 0,
            visible: false,
            anchored: false,
            font: 18.0,
            badge_bangla: true,
            badge_alpha: 255,
            dict,
            user,
        })
    });
}

pub fn stop() {
    with(|p| unsafe {
        DestroyWindow(p.hwnd);
        DestroyWindow(p.badge);
    });
    POPUP.with(|p| p.borrow_mut().take());
}

/// user.sqlite changed, or a dictionary was downloaded: reopen both (cheap) so new words show at once.
pub fn reload_user() {
    with(|p| {
        p.user = UserData::open(&likhon_store::user_db(&crate::settings::dir().0)).ok();
        p.dict = crate::settings::dictionary_path().and_then(|d| Dictionary::open(&d).ok());
    });
}

fn window(class: &str, layered: bool) -> HWND {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let name = wide(class);
        let wc = WNDCLASSW {
            style: CS_DROPSHADOW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: std::ptr::null_mut(),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: name.as_ptr(),
        };
        RegisterClassW(&wc);
        let ex = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | if layered { WS_EX_LAYERED | WS_EX_TRANSPARENT } else { 0 };
        let hwnd = CreateWindowExW(ex, name.as_ptr(), wide("Likhon").as_ptr(), WS_POPUP, 0, 0, 10, 10, std::ptr::null_mut(), std::ptr::null_mut(), hinst, std::ptr::null());
        if !hwnd.is_null() {
            let round: u32 = 3; // DWMWCP_ROUNDSMALL: radius 8 as in the design (Windows 11)
            DwmSetWindowAttribute(hwnd, 33, (&round as *const u32).cast(), 4);
            if layered {
                SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
            }
        }
        hwnd
    }
}

/// Recomputes and shows the window for the word being typed (called from the agent's message loop).
pub fn update(keys: &str, text: &str, auto: Option<&str>, s: &Settings, password: bool) {
    with(|p| fill(p, keys, text, auto, s, password));
}

fn fill(p: &mut Popup, keys: &str, text: &str, auto: Option<&str>, s: &Settings, password: bool) {
    let show = s.suggestions && !password && !text.is_empty() && text.chars().any(|c| ('\u{0980}'..='\u{09FF}').contains(&c));
    {
        if !show {
            hide_inner(p);
            return;
        }
        let limit = s.suggestion_count.clamp(1, 9);
        let mine = p.user.as_ref().map(|u| u.words_starting_with(text, limit)).unwrap_or_default();
        p.items = candidates(text, auto, &mine, p.dict.as_ref(), limit);
        p.selected = preselect(&p.items, text, p.dict.as_ref(), s.preselect_dict);
        // A word choice the person imported for these keys is picked first (added to the list if it is not there).
        if auto.is_none() {
            if let Some(choice) = p.user.as_ref().and_then(|u| u.get(likhon_store::List::Choices, keys)) {
                p.selected = choose(&mut p.items, choice, limit);
            }
        }
        p.keys = keys.to_string();
        p.anchored = s.suggest_anchored;
        p.font = s.font_size.clamp(13.0, 24.0) + 1.0;
        place(p);
        p.visible = true;
        unsafe {
            ShowWindow(p.hwnd, SW_SHOWNOACTIVATE);
            InvalidateRect(p.hwnd, std::ptr::null(), 0);
        }
    }
}

pub fn hide() {
    with(hide_inner);
}

fn hide_inner(p: &mut Popup) {
    if p.visible {
        unsafe { ShowWindow(p.hwnd, SW_HIDE) };
    }
    p.visible = false;
    p.items.clear();
}

pub fn visible() -> bool {
    with(|p| p.visible && !p.items.is_empty()).unwrap_or(false)
}

/// Up and Down move the selection (from the keyboard hook).
pub fn step(down: bool) {
    with(|p| {
        if p.items.is_empty() {
            return;
        }
        let n = p.items.len();
        // `n` means no row selected (next words): Down goes to the first row, Up to the last.
        p.selected = match (p.selected >= n, down) {
            (true, true) => 0,
            (true, false) => n - 1,
            (false, true) => (p.selected + 1) % n,
            (false, false) => (p.selected + n - 1) % n,
        };
        unsafe { InvalidateRect(p.hwnd, std::ptr::null(), 0) };
    });
}

/// The selected word, if it differs from what is on screen (so committing changes something).
pub fn selected_change(on_screen: &str) -> Option<String> {
    with(|p| p.items.get(p.selected).map(|c| c.word.clone()).filter(|w| w != on_screen)).flatten()
}

/// The word in row `i` (0-based), for the number keys 1 to 9.
pub fn word_at(i: usize) -> Option<String> {
    with(|p| p.items.get(i).map(|c| c.word.clone())).flatten()
}

// ---- Placement ------------------------------------------------------------------------------------------

/// The text caret of the focused window on screen, when the app reports one (Win32 and many others; Chrome and
/// Electron apps often do not).
pub fn caret() -> Option<RECT> {
    unsafe {
        let fg = GetForegroundWindow();
        let tid = GetWindowThreadProcessId(fg, std::ptr::null_mut());
        let mut gi: GUITHREADINFO = std::mem::zeroed();
        gi.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        if GetGUIThreadInfo(tid, &mut gi) == 0 || gi.hwndCaret.is_null() {
            return uia_caret();
        }
        let r = gi.rcCaret;
        let mut a = POINT { x: r.left, y: r.top };
        let mut b = POINT { x: r.right, y: r.bottom };
        ClientToScreen(gi.hwndCaret, &mut a);
        ClientToScreen(gi.hwndCaret, &mut b);
        Some(RECT { left: a.x, top: a.y, right: b.x.max(a.x + 1), bottom: b.y.max(a.y + 1) })
    }
}

/// UI Automation, made once on the agent thread (the caret fallback and the password check share it).
fn uia() -> Option<windows::Win32::UI::Accessibility::IUIAutomation> {
    thread_local! {
        static UIA: RefCell<Option<windows::Win32::UI::Accessibility::IUIAutomation>> = const { RefCell::new(None) };
    }
    UIA.with(|u| unsafe {
        use windows::Win32::System::Com::*;
        use windows::Win32::UI::Accessibility::*;
        let mut u = u.try_borrow_mut().ok()?;
        if u.is_none() {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            *u = CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok();
        }
        u.clone()
    })
}

/// The caret from UI Automation, for apps that do not report it to Windows (Chrome, Edge, Firefox, Electron and most
/// modern apps): the text pattern's caret range, else the focused box when it is one line high. Screen pixels.
fn uia_caret() -> Option<RECT> {
    use windows::Win32::System::Ole::{SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetUBound, SafeArrayUnaccessData};
    use windows::Win32::UI::Accessibility::*;
    unsafe {
        let el = uia()?.GetFocusedElement().ok()?;
        // The first rectangle of a range, or None when the range has no size (an empty caret range often has none).
        let first_rect = |r: &IUIAutomationTextRange| -> Option<(f64, f64, f64, f64)> {
            let sa = r.GetBoundingRectangles().ok()?;
            if sa.is_null() {
                return None;
            }
            let upper = SafeArrayGetUBound(sa, 1).unwrap_or(-1);
            let mut out = None;
            if upper >= 3 {
                let mut data: *mut core::ffi::c_void = std::ptr::null_mut();
                if SafeArrayAccessData(sa, &mut data).is_ok() && !data.is_null() {
                    let d = std::slice::from_raw_parts(data as *const f64, 4);
                    out = Some((d[0], d[1], d[2], d[3]));
                    let _ = SafeArrayUnaccessData(sa);
                }
            }
            let _ = SafeArrayDestroy(sa);
            out
        };
        if let Ok(tp) = el.GetCurrentPatternAs::<IUIAutomationTextPattern2>(UIA_TextPattern2Id) {
            let mut active = windows::core::BOOL(0);
            if let Ok(range) = tp.GetCaretRange(&mut active) {
                if let Some((x, y, _, h)) = first_rect(&range) {
                    return Some(RECT { left: x as i32, top: y as i32, right: x as i32 + 1, bottom: (y + h) as i32 });
                }
                // An empty range: take the character before the caret; the caret is at its right edge.
                if range.MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, -1).is_ok() {
                    if let Some((x, y, w, h)) = first_rect(&range) {
                        let right = (x + w) as i32;
                        return Some(RECT { left: right, top: y as i32, right: right + 1, bottom: (y + h) as i32 });
                    }
                }
            }
        }
        let b = el.CurrentBoundingRectangle().ok()?;
        let h = b.bottom - b.top;
        // A one-line box (a search field, a chat input): the badge and list go at its start. Bigger boxes give no
        // useful place, so nothing is shown rather than something in the wrong spot.
        (h > 0 && h <= 80 && b.right > b.left).then(|| RECT { left: b.left + 4, top: b.top, right: b.left + 5, bottom: b.bottom })
    }
}

/// Learns finished words into user.sqlite, and each word after the one before it (agent.rs checks the field first).
pub fn learn(words: &[(Option<String>, String)]) {
    with(|p| {
        if let Some(u) = p.user.as_ref() {
            for (prev, w) in words {
                if u.learn(w) {
                    if let Some(prev) = prev {
                        u.learn_pair(prev, w);
                    }
                }
            }
        }
    });
}

/// Between words: the words most often typed after `prev`, with no row selected (Space keeps typing a space; Up or
/// Down picks a row, then Space or Enter types it).
pub fn show_next(prev: &str, s: &Settings, password: bool) {
    with(|p| {
        let words = if password { Vec::new() } else { p.user.as_ref().map(|u| u.next_words(prev, s.next_count.clamp(1, 9))).unwrap_or_default() };
        if words.is_empty() {
            hide_inner(p);
            return;
        }
        p.items = words.into_iter().map(|word| Candidate { word, tag: Tag::Plain }).collect();
        p.selected = p.items.len(); // none
        p.keys.clear();
        p.anchored = s.suggest_anchored;
        p.font = s.font_size.clamp(13.0, 24.0) + 1.0;
        place(p);
        p.visible = true;
        unsafe {
            ShowWindow(p.hwnd, SW_SHOWNOACTIVATE);
            InvalidateRect(p.hwnd, std::ptr::null(), 0);
        }
    });
}

/// Whether the focused field is a password field: Some(true) or Some(false) when Windows says so, None when it cannot
/// tell. A Windows password box (ES_PASSWORD) first, then UI Automation's IsPassword for everything else (browsers,
/// modern apps). Unverified per app (Desktop/CLAUDE.md 5): see tests/compat. The answer is kept for half a second,
/// so the suggestion window can ask on every key without a cross-process call each time.
pub fn password_field() -> Option<bool> {
    thread_local! {
        static CACHE: std::cell::Cell<(u64, Option<bool>)> = const { std::cell::Cell::new((0, None)) };
    }
    let now = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() };
    let (at, v) = CACHE.with(|c| c.get());
    if at != 0 && now.saturating_sub(at) < 500 {
        return v;
    }
    let v = if in_password_box() {
        Some(true)
    } else {
        unsafe { uia().and_then(|u| u.GetFocusedElement().ok()).and_then(|el| el.CurrentIsPassword().ok()).map(|b| b.as_bool()) }
    };
    CACHE.with(|c| c.set((now, v)));
    v
}

/// Whether the focused control is a Windows password box (ES_PASSWORD).
pub fn in_password_box() -> bool {
    unsafe {
        let fg = GetForegroundWindow();
        let tid = GetWindowThreadProcessId(fg, std::ptr::null_mut());
        let mut gi: GUITHREADINFO = std::mem::zeroed();
        gi.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        if GetGUIThreadInfo(tid, &mut gi) == 0 || gi.hwndFocus.is_null() {
            return false;
        }
        const ES_PASSWORD: i32 = 0x20;
        let mut class = [0u16; 32];
        let n = GetClassNameW(gi.hwndFocus, class.as_mut_ptr(), 32) as usize;
        let name = String::from_utf16_lossy(&class[..n]).to_ascii_lowercase();
        name.contains("edit") && GetWindowLongW(gi.hwndFocus, GWL_STYLE) & ES_PASSWORD != 0
    }
}

fn place(p: &mut Popup) {
    unsafe {
        let k = GetDpiForWindow(p.hwnd).max(96) as f32 / 96.0;
        let h = HEAD + PAD * 2.0 + ROW * p.items.len() as f32;
        let (w, hh) = ((W * k).round() as i32, (h * k).round() as i32);
        // Anchored means under the top bar; with the top bar closed the list follows the caret instead of a corner.
        let bar = FindWindowW(wide("LikhonTopBar").as_ptr(), std::ptr::null());
        let bar_shows = !bar.is_null() && IsWindowVisible(bar) != 0;
        let at = if p.anchored && bar_shows { None } else { caret() };
        let (x, y, above) = match at {
            Some(r) => (r.left, r.bottom + (4.0 * k) as i32, r.top - (4.0 * k) as i32),
            None => {
                // Anchored, or the app does not report its caret: under the top bar, else bottom right.
                let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                let bar = FindWindowW(wide("LikhonTopBar").as_ptr(), std::ptr::null());
                if !bar.is_null() && IsWindowVisible(bar) != 0 && GetWindowRect(bar, &mut r) != 0 {
                    (r.left, r.bottom + (6.0 * k) as i32, r.top - (6.0 * k) as i32)
                } else {
                    let mut pt = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut pt);
                    let wa = work_area(pt);
                    (wa.right - w - 24, wa.bottom - hh - 24, wa.bottom - hh - 24)
                }
            }
        };
        let wa = work_area(POINT { x, y });
        // Near the bottom of the screen the window flips above the caret (§4 note).
        let y = if y + hh > wa.bottom { above - hh } else { y };
        let x = x.clamp(wa.left, (wa.right - w).max(wa.left));
        SetWindowPos(p.hwnd, HWND_TOPMOST, x, y.max(wa.top), w, hh, SWP_NOACTIVATE);
    }
}

fn work_area(pt: POINT) -> RECT {
    unsafe {
        let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(mon, &mut mi);
        mi.rcWork
    }
}

// ---- Caret badge ----------------------------------------------------------------------------------------

/// Shows অ (Bangla) or A (English) beside the caret, if the app reports one.
pub fn badge(bangla: bool, enabled: bool) {
    if !enabled {
        return;
    }
    let Some(r) = caret() else { return };
    with(|p| unsafe {
        let k = GetDpiForWindow(p.badge).max(96) as f32 / 96.0;
        let s = (20.0 * k).round() as i32;
        p.badge_bangla = bangla;
        p.badge_alpha = 255;
        SetLayeredWindowAttributes(p.badge, 0, 255, LWA_ALPHA);
        SetWindowPos(p.badge, HWND_TOPMOST, r.right + (3.0 * k) as i32, r.bottom - s / 2, s, s, SWP_NOACTIVATE);
        ShowWindow(p.badge, SW_SHOWNOACTIVATE);
        InvalidateRect(p.badge, std::ptr::null(), 0);
        KillTimer(p.badge, TIMER_FADE);
        SetTimer(p.badge, TIMER_BADGE, BADGE_MS, None);
    });
}

// ---- Drawing --------------------------------------------------------------------------------------------

fn target(gfx: &Gfx, hwnd: HWND, slot: &mut Option<ID2D1HwndRenderTarget>) -> Option<ID2D1HwndRenderTarget> {
    unsafe {
        let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetClientRect(hwnd, &mut rc);
        let px = D2D_SIZE_U { width: (rc.right - rc.left).max(1) as u32, height: (rc.bottom - rc.top).max(1) as u32 };
        if let Some(t) = slot.as_ref() {
            let _ = t.Resize(&px);
        } else {
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                // Software, like the top bar: no GPU driver loaded into the agent.
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT { format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN, alphaMode: D2D1_ALPHA_MODE_UNKNOWN },
                dpiX: 0.0,
                dpiY: 0.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let hprops = D2D1_HWND_RENDER_TARGET_PROPERTIES { hwnd: windows::Win32::Foundation::HWND(hwnd), pixelSize: px, presentOptions: D2D1_PRESENT_OPTIONS_NONE };
            *slot = gfx.d2d.CreateHwndRenderTarget(&props, &hprops).ok();
        }
        let dpi = GetDpiForWindow(hwnd) as f32;
        if let Some(t) = slot.as_ref() {
            t.SetDpi(dpi, dpi);
        }
        slot.clone()
    }
}

fn paint_list(p: &mut Popup) {
    let th = topbar::current_theme();
    let Some(rt) = target(&p.gfx, p.hwnd, &mut p.target) else { return };
    let s = crate::agent::settings();
    let lang = s.language;
    unsafe {
        rt.BeginDraw();
        rt.Clear(Some(&topbar::rgb(th.bg)));
        let pt = Painter { rt: &rt, gfx: &p.gfx };
        let w = W;
        let h = HEAD + PAD * 2.0 + ROW * p.items.len() as f32;
        pt.round_rect(D2D_RECT_F { left: 0.0, top: 0.0, right: w, bottom: h }, 8.0, None, Some(th.border));
        // Header: the typed letters, and the pin (anchored).
        pt.text(&p.keys, 10.0, 0.0, HEAD, 13.0, 400, th.text2);
        pt.round_rect(D2D_RECT_F { left: w - 28.0, top: 4.0, right: w - 6.0, bottom: 26.0 }, 5.0, if p.anchored { Some(th.sel) } else { None }, None);
        pt.icon(crate::icons::PIN, w - 24.0, 8.0, 14.0, if p.anchored { th.accent } else { th.text2 }, th.border);
        pt.round_rect(D2D_RECT_F { left: 0.0, top: HEAD - 1.0, right: w, bottom: HEAD }, 0.0, Some(th.border), None);
        for (i, c) in p.items.iter().enumerate() {
            let top = HEAD + PAD + ROW * i as f32;
            if i == p.selected {
                pt.round_rect(D2D_RECT_F { left: PAD, top, right: w - PAD, bottom: top + ROW }, 5.0, Some(th.sel), None);
            }
            let n = if lang == crate::text::Lang::Bn { char::from_u32(0x09E7 + i as u32).map(String::from).unwrap_or_default() } else { (i + 1).to_string() };
            pt.text(&n, 14.0, top, top + ROW, 12.0, 400, th.text2);
            let tag = match c.tag {
                Tag::Auto => crate::text::t("app.suggest.auto", lang, &[]),
                Tag::Mine => crate::text::t("app.suggest.mine", lang, &[]),
                _ => String::new(),
            };
            let tag_w = if tag.is_empty() { 0.0 } else { topbar::text_layout(&p.gfx.dw, &tag, 11.0, 400).map(|x| x.1).unwrap_or(0.0) };
            pt.text_clipped(&c.word, &topbar::face(), 34.0, top, top + ROW, w - 46.0 - tag_w, p.font, 400, th.text);
            if !tag.is_empty() {
                pt.text(&tag, w - 12.0 - tag_w, top, top + ROW, 11.0, 400, th.text2);
            }
        }
        if rt.EndDraw(None, None).is_err() {
            p.target = None;
        }
    }
}

fn paint_badge(p: &mut Popup) {
    let th = topbar::current_theme();
    let Some(rt) = target(&p.gfx, p.badge, &mut p.badge_target) else { return };
    unsafe {
        rt.BeginDraw();
        let (bg, ink) = if p.badge_bangla { (th.accent, th.on_accent) } else { (th.bg, th.text) };
        rt.Clear(Some(&topbar::rgb(bg)));
        let pt = Painter { rt: &rt, gfx: &p.gfx };
        if !p.badge_bangla {
            pt.round_rect(D2D_RECT_F { left: 0.0, top: 0.0, right: 20.0, bottom: 20.0 }, 5.0, None, Some(th.border));
        }
        let label = if p.badge_bangla { "অ" } else { "A" };
        let lw = topbar::text_layout(&p.gfx.dw, label, 12.0, 700).map(|x| x.1).unwrap_or(8.0);
        pt.text(label, (20.0 - lw) / 2.0, 0.0, 20.0, 12.0, 700, ink);
        if rt.EndDraw(None, None).is_err() {
            p.badge_target = None;
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let is_badge = with(|p| p.badge == hwnd);
    let Some(is_badge) = is_badge else { return DefWindowProcW(hwnd, msg, wparam, lparam) };
    match msg {
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            BeginPaint(hwnd, &mut ps);
            with(|p| if is_badge { paint_badge(p) } else { paint_list(p) });
            EndPaint(hwnd, &ps);
            0
        }
        WM_ERASEBKGND => 1,
        WM_LBUTTONUP if !is_badge => {
            // A click on a row commits it; a click on the pin switches between following the cursor and anchored.
            let x = (lparam & 0xFFFF) as i16 as f32;
            let y = ((lparam >> 16) & 0xFFFF) as i16 as f32;
            let k = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
            let (x, y) = (x / k, y / k);
            if y < HEAD && x > W - 30.0 {
                crate::agent::toggle_anchored();
            } else if y >= HEAD + PAD {
                let i = ((y - HEAD - PAD) / ROW) as usize;
                let word = with(|p| p.items.get(i).map(|c| c.word.clone())).flatten();
                if let Some(w) = word {
                    crate::agent::commit_word(&w);
                }
            }
            0
        }
        WM_TIMER if wparam == TIMER_BADGE => {
            KillTimer(hwnd, TIMER_BADGE);
            SetTimer(hwnd, TIMER_FADE, FADE_MS / 8, None);
            0
        }
        WM_TIMER if wparam == TIMER_FADE => {
            let a = with(|p| {
                p.badge_alpha = p.badge_alpha.saturating_sub(32);
                p.badge_alpha
            })
            .unwrap_or(0);
            if a == 0 {
                KillTimer(hwnd, TIMER_FADE);
                ShowWindow(hwnd, SW_HIDE);
            } else {
                SetLayeredWindowAttributes(hwnd, 0, a, LWA_ALPHA);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
