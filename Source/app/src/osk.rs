//! The on-screen keyboard and its Letters panel (Reference/design-v2/Likhon Desktop.dc.html §8).
//!
//! A native window that never takes focus (WS_EX_NOACTIVATE, MA_NOACTIVATE): clicking a key types its character into
//! the app being typed in. The keys show the layout in use (a fixed layout's file, or what each key gives alone in
//! Phonetic), at the level chosen in the header (Normal, Shift, AltGr). Letters switches the body to a grid of
//! consonants, vowels, vowel signs, signs, conjuncts and digits. Opened from Settings › Tools or Ctrl + Alt + K.

use crate::agent;
use crate::text::{t, wide};
use crate::topbar::{self, Gfx, Painter};
use likhon_engine::fixed::{KeyPress, Layout};
use std::cell::RefCell;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const HEAD: f32 = 36.0;
const KEY: f32 = 46.0;
const GAP: f32 = 5.0;
const PAD: f32 = 12.0;
const ROWS: [&str; 4] = ["`1234567890-=", "qwertyuiop[]\\", "asdfghjkl;'", "zxcvbnm,./"];
const INDENT: [f32; 4] = [0.0, 24.0, 36.0, 60.0];
/// Letters panel tabs (§8): characters to type, not interface text.
const LETTERS: [(&str, &str); 6] = [
    ("app.osk.cons", "ক খ গ ঘ ঙ চ ছ জ ঝ ঞ ট ঠ ড ঢ ণ ত থ দ ধ ন প ফ ব ভ ম য র ল শ ষ স হ ড় ঢ় য় ৎ"),
    ("app.osk.vow", "অ আ ই ঈ উ ঊ ঋ এ ঐ ও ঔ"),
    ("app.osk.kar", "া ি ী ু ূ ৃ ে ৈ ো ৌ"),
    ("app.osk.sign", "ং ঃ ঁ ্ । ॥ ৳ ঽ"),
    ("app.osk.conj", "ক্ষ জ্ঞ ঙ্গ ঞ্চ ন্ত ন্দ স্ত ত্র ক্র প্র শ্র ম্ব ন্ড ষ্ট ক্ত হ্ম"),
    ("app.osk.dig", "০ ১ ২ ৩ ৪ ৫ ৬ ৭ ৮ ৯"),
];

#[derive(Clone, Copy, PartialEq)]
enum Level {
    Normal,
    Shift,
    AltGr,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Hit {
    Title,
    Level(u8),
    Layout,
    Letters,
    Close,
    Key(char),
    Space,
    Back,
    Tab(usize),
    Letter(usize),
}

struct Osk {
    gfx: Gfx,
    hwnd: HWND,
    target: Option<ID2D1HwndRenderTarget>,
    level: Level,
    letters: bool,
    tab: usize,
    hits: Vec<(Hit, D2D_RECT_F)>,
    drag: Option<(i32, i32)>,
    pressed: Option<Hit>,
}

thread_local! {
    static OSK: RefCell<Option<Osk>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut Osk) -> R) -> Option<R> {
    OSK.with(|o| o.try_borrow_mut().ok()?.as_mut().map(f))
}

/// Opens the keyboard, or closes it if it is open (Ctrl + Alt + K toggles).
pub fn toggle() {
    if with(|o| unsafe { IsWindowVisible(o.hwnd) != 0 }).unwrap_or(false) {
        close();
        return;
    }
    if with(|_| ()).is_none() {
        let Some(gfx) = Gfx::new() else { return };
        let hwnd = create();
        if hwnd.is_null() {
            return;
        }
        OSK.with(|o| *o.borrow_mut() = Some(Osk { gfx, hwnd, target: None, level: Level::Normal, letters: false, tab: 0, hits: Vec::new(), drag: None, pressed: None }));
    }
    with(|o| unsafe {
        let (w, h) = size(o.letters);
        let k = GetDpiForWindow(o.hwnd).max(96) as f32 / 96.0;
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        // Bottom centre of the screen with the mouse, above the taskbar.
        let mon = windows_sys::Win32::Graphics::Gdi::MonitorFromPoint(pt, windows_sys::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST);
        let mut mi: windows_sys::Win32::Graphics::Gdi::MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<windows_sys::Win32::Graphics::Gdi::MONITORINFO>() as u32;
        windows_sys::Win32::Graphics::Gdi::GetMonitorInfoW(mon, &mut mi);
        let r = mi.rcWork;
        let (pw, ph) = ((w * k) as i32, (h * k) as i32);
        SetWindowPos(o.hwnd, HWND_TOPMOST, (r.left + r.right - pw) / 2, r.bottom - ph - (16.0 * k) as i32, pw, ph, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        InvalidateRect(o.hwnd, std::ptr::null(), 0);
    });
}

/// The layout or language changed (top bar, Settings, the next-layout key): draw the keys again at once.
pub fn refresh() {
    with(|o| unsafe {
        if IsWindowVisible(o.hwnd) != 0 {
            InvalidateRect(o.hwnd, std::ptr::null(), 0);
        }
    });
}

pub fn close() {
    with(|o| unsafe { ShowWindow(o.hwnd, SW_HIDE) });
}

fn size(letters: bool) -> (f32, f32) {
    let w = PAD * 2.0 + 13.0 * KEY + 12.0 * GAP + 80.0;
    let body = if letters { 34.0 + 5.0 * (40.0 + 4.0) } else { 5.0 * (KEY + GAP) };
    (w, HEAD + PAD + body + PAD)
}

fn create() -> HWND {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let name = wide("LikhonKeyboard");
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
        let hwnd = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, name.as_ptr(), wide("Likhon").as_ptr(), WS_POPUP, 0, 0, 10, 10, std::ptr::null_mut(), std::ptr::null_mut(), hinst, std::ptr::null());
        if !hwnd.is_null() {
            let round: u32 = 2; // radius 12 for windows (v2 §1), Windows 11
            DwmSetWindowAttribute(hwnd, 33, (&round as *const u32).cast(), 4);
        }
        hwnd
    }
}

/// What a key types at a level: the fixed layout's file, or in Phonetic what the key gives on its own.
fn key_text(layout: Option<&Layout>, key: char, level: Level) -> String {
    match layout {
        Some(l) => l.lookup(KeyPress { key, shift: level == Level::Shift, altgr: level == Level::AltGr }).unwrap_or("").to_string(),
        None => {
            if level == Level::AltGr || !key.is_ascii_alphanumeric() {
                return String::new();
            }
            let k = if level == Level::Shift { key.to_ascii_uppercase() } else { key };
            if k.is_ascii_digit() {
                return likhon_engine::tools::bn_digits(&k.to_string());
            }
            likhon_engine::phonetic(&k.to_string())
        }
    }
}

fn shown(s: &str) -> String {
    // Vowel signs and the hasanta sit on a dotted circle so they can be seen alone.
    if s.chars().count() == 1 && s.chars().all(|c| matches!(c, '\u{09BE}'..='\u{09CC}' | '\u{09D7}' | '\u{0981}'..='\u{0983}' | '\u{09CD}')) {
        format!("◌{s}")
    } else {
        s.to_string()
    }
}

fn paint(o: &mut Osk) {
    let th = topbar::current_theme();
    let s = agent::settings();
    let lang = s.language;
    let layout = crate::layouts::load(&s.layout);
    let (w, _) = size(o.letters);
    unsafe {
        if o.target.is_none() {
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(o.hwnd, &mut rc);
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT { format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN, alphaMode: D2D1_ALPHA_MODE_UNKNOWN },
                dpiX: 0.0,
                dpiY: 0.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let px = D2D_SIZE_U { width: (rc.right - rc.left).max(1) as u32, height: (rc.bottom - rc.top).max(1) as u32 };
            let hp = D2D1_HWND_RENDER_TARGET_PROPERTIES { hwnd: windows::Win32::Foundation::HWND(o.hwnd), pixelSize: px, presentOptions: D2D1_PRESENT_OPTIONS_NONE };
            o.target = o.gfx.d2d.CreateHwndRenderTarget(&props, &hp).ok();
        }
        let Some(rt) = o.target.clone() else { return };
        let dpi = GetDpiForWindow(o.hwnd) as f32;
        rt.SetDpi(dpi, dpi);
        let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetClientRect(o.hwnd, &mut rc);
        let _ = rt.Resize(&D2D_SIZE_U { width: (rc.right - rc.left).max(1) as u32, height: (rc.bottom - rc.top).max(1) as u32 });
        rt.BeginDraw();
        rt.Clear(Some(&topbar::rgb(th.bg)));
        let p = Painter { rt: &rt, gfx: &o.gfx };
        let mut hits: Vec<(Hit, D2D_RECT_F)> = Vec::new();
        let r = |l: f32, t: f32, rr: f32, b: f32| D2D_RECT_F { left: l, top: t, right: rr, bottom: b };

        // Header: title, level picker, layout name, Letters, close.
        hits.push((Hit::Title, r(0.0, 0.0, w, HEAD)));
        p.icon(crate::icons::KEYBOARD, 12.0, 10.0, 16.0, th.accent, th.sel);
        let tw = p.text(&t("app.osk.title", lang, &[]), 36.0, 0.0, HEAD, 13.0, 400, th.text);
        let mut x = 36.0 + tw + 16.0;
        if !o.letters {
            let labels = [t("app.osk.normal", lang, &[]), "Shift".to_string(), "AltGr".to_string()];
            let seg_w: Vec<f32> = labels.iter().map(|l| topbar::text_layout(&o.gfx.dw, l, 12.0, 400).map(|m| m.1).unwrap_or(30.0) + 20.0).collect();
            p.round_rect(r(x, 6.0, x + seg_w.iter().sum::<f32>() + 4.0, 30.0), 6.0, Some(th.soft), None);
            let mut sx = x + 2.0;
            for (i, l) in labels.iter().enumerate() {
                let on = o.level == [Level::Normal, Level::Shift, Level::AltGr][i];
                if on {
                    p.round_rect(r(sx, 8.0, sx + seg_w[i], 28.0), 4.0, Some(th.bg), None);
                }
                p.text(l, sx + 10.0, 8.0, 28.0, 12.0, if on { 600 } else { 400 }, th.text);
                hits.push((Hit::Level(i as u8), r(sx, 6.0, sx + seg_w[i], 30.0)));
                sx += seg_w[i];
            }
            // The layout in use, as a picker: the top bar's layout list opens under it, and a pick changes both.
            x = sx + 8.0;
            let name = topbar::layout_name(&s, lang);
            let nw = topbar::text_layout(&o.gfx.dw, &name, 12.0, 400).map(|m| m.1).unwrap_or(40.0);
            let lr = r(x, 6.0, x + 10.0 + nw + 4.0 + 14.0 + 8.0, 30.0);
            if o.pressed == Some(Hit::Layout) {
                p.round_rect(lr, 6.0, Some(th.soft), None);
            }
            p.text(&name, x + 10.0, 6.0, 30.0, 12.0, 400, th.text);
            p.icon(crate::icons::CHEVRON_DOWN, x + 10.0 + nw + 4.0, 11.0, 14.0, th.accent, th.sel);
            hits.push((Hit::Layout, lr));
        }
        // Letters toggle and close, on the right.
        let lw = topbar::text_layout(&o.gfx.dw, &t("app.osk.letters", lang, &[]), 12.0, 600).map(|m| m.1).unwrap_or(40.0) + 30.0;
        let lx = w - 44.0 - lw - 4.0;
        p.round_rect(r(lx, 6.0, lx + lw, 30.0), 5.0, if o.letters { Some(th.sel) } else { None }, None);
        p.text(&t("app.osk.letters", lang, &[]), lx + 22.0, 6.0, 30.0, 12.0, 600, th.text);
        p.text("অ", lx + 6.0, 6.0, 30.0, 12.0, 600, th.accent);
        hits.push((Hit::Letters, r(lx, 6.0, lx + lw, 30.0)));
        p.icon(crate::icons::X, w - 31.0, 11.0, 14.0, th.text2, th.border);
        hits.push((Hit::Close, r(w - 44.0, 0.0, w, HEAD)));
        p.round_rect(r(0.0, HEAD - 1.0, w, HEAD), 0.0, Some(th.border), None);

        let top = HEAD + PAD;
        if o.letters {
            let mut tx = PAD;
            for (i, (id, _)) in LETTERS.iter().enumerate() {
                let label = t(id, lang, &[]);
                let tw = topbar::text_layout(&o.gfx.dw, &label, 12.0, 400).map(|m| m.1).unwrap_or(30.0) + 18.0;
                if i == o.tab {
                    p.round_rect(r(tx, top, tx + tw, top + 26.0), 5.0, Some(th.sel), None);
                }
                p.text(&label, tx + 9.0, top, top + 26.0, 12.0, if i == o.tab { 600 } else { 400 }, th.text);
                hits.push((Hit::Tab(i), r(tx, top, tx + tw, top + 26.0)));
                tx += tw + 4.0;
            }
            let cols = 14.0;
            let cw = (w - PAD * 2.0 - (cols - 1.0) * 4.0) / cols;
            for (i, ch) in LETTERS[o.tab].1.split(' ').enumerate() {
                let (cx, cy) = (PAD + (i % 14) as f32 * (cw + 4.0), top + 34.0 + (i / 14) as f32 * 44.0);
                let kr = r(cx, cy, cx + cw, cy + 40.0);
                p.round_rect(kr, 6.0, Some(if o.pressed == Some(Hit::Letter(i)) { th.sel } else { th.bg }), Some(th.border));
                let s = shown(ch);
                let tw = topbar::text_layout(&o.gfx.dw, &s, 19.0, 400).map(|m| m.1).unwrap_or(10.0);
                p.text(&s, cx + (cw - tw) / 2.0, cy, cy + 40.0, 19.0, 400, th.text);
                hits.push((Hit::Letter(i), kr));
            }
        } else {
            for (ri, row) in ROWS.iter().enumerate() {
                let y = top + ri as f32 * (KEY + GAP);
                for (ci, k) in row.chars().enumerate() {
                    let kx = PAD + INDENT[ri] + ci as f32 * (KEY + GAP);
                    let kr = r(kx, y, kx + KEY, y + KEY);
                    p.round_rect(kr, 6.0, Some(if o.pressed == Some(Hit::Key(k)) { th.sel } else { th.bg }), Some(th.border));
                    p.text(&k.to_ascii_uppercase().to_string(), kx + 5.0, y + 1.0, y + 14.0, 10.0, 400, th.text2);
                    let main = shown(&key_text(layout.as_ref(), k, o.level));
                    if !main.is_empty() {
                        let size = if main.chars().count() > 2 { 15.0 } else { 18.0 };
                        let tw = topbar::text_layout(&o.gfx.dw, &main, size, 400).map(|m| m.1).unwrap_or(10.0);
                        p.text(&main, kx + (KEY - tw) / 2.0, y + 8.0, y + KEY, size, 400, th.text);
                    }
                    hits.push((Hit::Key(k), kr));
                }
            }
            // Space and Backspace.
            let y = top + 4.0 * (KEY + GAP);
            let sx = PAD + 3.0 * (KEY + GAP) + 40.0;
            let space = r(sx, y, sx + 6.0 * (KEY + GAP), y + KEY);
            p.round_rect(space, 6.0, Some(if o.pressed == Some(Hit::Space) { th.sel } else { th.soft }), Some(th.border));
            hits.push((Hit::Space, space));
            let bx = space.right + GAP;
            let back = r(bx, y, bx + 2.0 * KEY, y + KEY);
            p.round_rect(back, 6.0, Some(if o.pressed == Some(Hit::Back) { th.sel } else { th.soft }), Some(th.border));
            let bw = topbar::text_layout(&o.gfx.dw, "⌫", 16.0, 400).map(|m| m.1).unwrap_or(10.0);
            p.text("⌫", bx + (2.0 * KEY - bw) / 2.0, y, y + KEY, 16.0, 400, th.text2);
            hits.push((Hit::Back, back));
        }
        o.hits = hits;
        if rt.EndDraw(None, None).is_err() {
            o.target = None;
        }
    }
}

fn hit_at(o: &Osk, lparam: LPARAM) -> Option<Hit> {
    let k = unsafe { GetDpiForWindow(o.hwnd) }.max(96) as f32 / 96.0;
    let x = (lparam & 0xFFFF) as i16 as f32 / k;
    let y = ((lparam >> 16) & 0xFFFF) as i16 as f32 / k;
    // Later entries are on top (keys over the title strip are not possible; the close box is over the title).
    o.hits.iter().rev().find(|(_, r)| x >= r.left && x < r.right && y >= r.top && y < r.bottom).map(|(h, _)| *h)
}

fn activate(hit: Hit) {
    let s = agent::settings();
    match hit {
        Hit::Close => close(),
        Hit::Level(i) => {
            with(|o| o.level = [Level::Normal, Level::Shift, Level::AltGr][i as usize]);
        }
        Hit::Layout => {
            // The button's rectangle on screen, for the list to open under it.
            let at = with(|o| unsafe {
                let r = o.hits.iter().find(|(h, _)| *h == Hit::Layout).map(|(_, r)| *r)?;
                let k = GetDpiForWindow(o.hwnd).max(96) as f32 / 96.0;
                let mut wr = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetWindowRect(o.hwnd, &mut wr);
                Some((RECT { left: wr.left + (r.left * k) as i32, top: wr.top + (r.top * k) as i32, right: wr.left + (r.right * k) as i32, bottom: wr.top + (r.bottom * k) as i32 }, o.hwnd))
            })
            .flatten();
            if let Some((rect, hwnd)) = at {
                topbar::open_layout_menu(rect, hwnd);
            }
        }
        Hit::Letters => {
            with(|o| {
                o.letters = !o.letters;
                o.target = None;
            });
            // Resize for the other body, keeping the bottom edge where it was.
            with(|o| unsafe {
                let mut wr = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetWindowRect(o.hwnd, &mut wr);
                let k = GetDpiForWindow(o.hwnd).max(96) as f32 / 96.0;
                let (w, h) = size(o.letters);
                let (pw, ph) = ((w * k) as i32, (h * k) as i32);
                SetWindowPos(o.hwnd, HWND_TOPMOST, wr.left, wr.bottom - ph, pw, ph, SWP_NOACTIVATE);
            });
        }
        Hit::Tab(i) => {
            with(|o| o.tab = i);
        }
        Hit::Key(k) => {
            let level = with(|o| o.level).unwrap_or(Level::Normal);
            let text = key_text(crate::layouts::load(&s.layout).as_ref(), k, level);
            if !text.is_empty() {
                agent::type_text(&text);
            }
            // Shift is for one key, as on a phone keyboard.
            with(|o| {
                if o.level == Level::Shift {
                    o.level = Level::Normal;
                }
            });
        }
        Hit::Letter(i) => {
            let tab = with(|o| o.tab).unwrap_or(0);
            if let Some(ch) = LETTERS[tab].1.split(' ').nth(i) {
                agent::type_text(ch);
            }
        }
        Hit::Space => agent::type_text(" "),
        Hit::Back => agent::press_backspace(),
        Hit::Title => {}
    }
    with(|o| unsafe { InvalidateRect(o.hwnd, std::ptr::null(), 0) });
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT, // keys type into the app, which keeps the focus
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            BeginPaint(hwnd, &mut ps);
            with(paint);
            EndPaint(hwnd, &ps);
            0
        }
        WM_ERASEBKGND => 1,
        WM_LBUTTONDOWN => {
            let h = with(|o| hit_at(o, lparam)).flatten();
            if h == Some(Hit::Title) {
                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);
                let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                GetWindowRect(hwnd, &mut r);
                with(|o| o.drag = Some((pt.x - r.left, pt.y - r.top)));
                windows_sys::Win32::UI::Input::KeyboardAndMouse::SetCapture(hwnd);
            } else {
                with(|o| o.pressed = h);
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        WM_MOUSEMOVE => {
            if let Some((dx, dy)) = with(|o| o.drag).flatten() {
                let mut pt = POINT { x: 0, y: 0 };
                GetCursorPos(&mut pt);
                SetWindowPos(hwnd, std::ptr::null_mut(), pt.x - dx, pt.y - dy, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
            }
            0
        }
        WM_LBUTTONUP => {
            let dragging = with(|o| o.drag.take().is_some()).unwrap_or(false);
            if dragging {
                windows_sys::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                return 0;
            }
            let (down, up) = with(|o| (o.pressed.take(), hit_at(o, lparam))).unwrap_or((None, None));
            if down.is_some() && down == up {
                activate(down.unwrap());
            } else {
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phonetic_keys_show_what_they_type_alone() {
        assert_eq!(key_text(None, 'k', Level::Normal), "ক");
        assert_eq!(key_text(None, '1', Level::Normal), "১");
        assert_eq!(key_text(None, 'k', Level::AltGr), "");
        let probhat = crate::layouts::load("probhat").unwrap();
        assert_eq!(key_text(Some(&probhat), 'k', Level::Normal), "ক");
        assert_eq!(shown("া"), "◌া");
    }

    #[test]
    fn letters_tabs_have_names() {
        assert!(LETTERS.iter().all(|(id, chars)| id.starts_with("app.osk.") && !chars.is_empty()));
    }
}
