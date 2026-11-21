//! Word suggestions while typing with the Likhon keyboard (owner decision 2026-10-24: in Windows-keyboard mode the
//! list shows whether or not likhon.exe runs, as other Bangla keyboards do). The keyboard reads the dictionary and the
//! person's My words and word choices itself, read-only (likhon-store: Dictionary, UserData::open_read_only), ranks
//! them with the same rules as likhon.exe (likhon-store/src/rank.rs) and draws its own small window in the app.
//!
//! The window never takes focus (WS_EX_NOACTIVATE, MA_NOACTIVATE); Up and Down move the selection, Space and Enter take
//! it, Escape closes it, a click on a row takes that row. It follows Likhon's settings: suggestions on or off, how
//! many, which row starts selected, anchored under the top bar or beside the word, the font and size, and the theme's
//! colours (light or dark as Settings or Windows say). Nothing is written here: a finished word goes to likhon.exe to be
//! learned when it runs (COMMIT, app/src/tsfbridge.rs), and never from a password field.
//! Drawn with GDI: the host app's process gets no Direct2D or web view from us.

use likhon_store::rank::{candidates, choose, preselect, Candidate};
use likhon_store::{Dictionary, List as UserList, UserData};
use serde_json::Value;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::{GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows_core::{w, PCWSTR};

const AGENT_CLASS: PCWSTR = w!("LikhonAgentWindow");
const CLASS: PCWSTR = w!("LikhonKeyboardList");
/// COMMIT to likhon.exe (app/src/tsfbridge.rs).
const TAG_COMMIT: usize = 0x4C4B_0004;

/// Sizes in DIPs, as likhon.exe's suggestion window (app/src/suggest.rs).
const W: f32 = 220.0;
const HEAD: f32 = 30.0;
const ROW: f32 = 34.0;
const PAD: f32 = 3.0;

/// The built-in themes and the colour tokens they refer to (the same files the exe and Settings use).
const BUILT_IN: [(&str, &str); 3] = [
    ("default", include_str!("../../assets/themes/default.json")),
    ("paper", include_str!("../../assets/themes/paper.json")),
    ("green", include_str!("../../assets/themes/green.json")),
];
const TOKENS: &str = include_str!("../../../Reference/design-tokens.json");

/// What the settings say, read again with settings.json (configure).
#[derive(Clone)]
struct Opts {
    on: bool,
    count: usize,
    prefer_dict: bool,
    anchored: bool,
    face: String,
    size: f32,
    colors: Colors,
}

#[derive(Clone, Copy)]
struct Colors {
    surface: u32,
    line: u32,
    ink: u32,
    muted: u32,
    sel: u32,
}

impl Default for Opts {
    fn default() -> Self {
        Opts { on: true, count: 5, prefer_dict: true, anchored: false, face: "Nirmala UI".into(), size: 18.0, colors: Colors { surface: 0xFFFFFF, line: 0xD8DDE3, ink: 0x1A1D21, muted: 0x5C6570, sel: 0xE3ECFB } }
    }
}

#[derive(Default)]
struct List {
    hwnd: Option<HWND>,
    pick: Option<Rc<dyn Fn(String)>>,
    opts: Opts,
    dict: Option<Dictionary>,
    user: Option<UserData>,
    keys: String,
    items: Vec<Candidate>,
    sel: usize,
}

thread_local! {
    static LIST: RefCell<List> = RefCell::new(List::default());
}

fn module() -> HINSTANCE {
    let mut h = HMODULE::default();
    unsafe {
        let _ = GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, PCWSTR(module as *const u16), &mut h);
    }
    HINSTANCE(h.0)
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 0xRRGGBB to a GDI colour (0x00BBGGRR).
fn rgb(c: u32) -> COLORREF {
    COLORREF(((c & 0xFF) << 16) | (c & 0xFF00) | ((c >> 16) & 0xFF))
}

// ---- Settings, theme, dictionary -----------------------------------------------------------------------------

fn windows_dark() -> bool {
    let mut v = 1u32;
    let mut size = 4u32;
    let r = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"), w!("AppsUseLightTheme"), RRF_RT_REG_DWORD, None, Some(&mut v as *mut u32 as *mut _), Some(&mut size))
    };
    r.is_ok() && v == 0
}

/// "#RRGGBB" or "@token" (design-tokens.json, with aliases) to 0xRRGGBB.
fn color(v: &str, mode: &str, tokens: &Value) -> Option<u32> {
    let mut v = v.to_string();
    for _ in 0..4 {
        if let Some(name) = v.strip_prefix('@') {
            let t = &tokens["color"][name];
            v = t[mode].as_str().or_else(|| t["alias"].as_str().map(|a| a)).map(|s| if s.starts_with('#') { s.to_string() } else { format!("@{s}") })?;
        } else {
            break;
        }
    }
    let hex = v.strip_prefix('#').filter(|h| h.len() == 6)?;
    u32::from_str_radix(hex, 16).ok()
}

fn theme_colors(text: &str, dark: bool) -> Option<Colors> {
    let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let tokens: Value = serde_json::from_str(TOKENS).ok()?;
    let mode = if dark { "dark" } else { "light" };
    let c = &v[mode];
    let get = |k: &str| c[k].as_str().and_then(|s| color(s, mode, &tokens));
    Some(Colors { surface: get("surface")?, line: get("line")?, ink: get("ink")?, muted: get("muted")?, sel: get("sel")? })
}

/// Theme ids saved by earlier builds (as app/src/topbar.rs canonical()).
fn canonical(id: &str) -> &str {
    match id {
        "default-light" | "default-dark" => "default",
        "night-green" => "green",
        "contrast-light" => "contrast",
        "avro-aero7-dark" => "aero",
        "avro-iavro" => "maroon",
        other => other.strip_prefix("avro-").unwrap_or(other),
    }
}

/// The dictionary: the second line of datadir.txt (written by likhon.exe when it adds the keyboard), else dict/
/// beside the data folder.
fn dictionary_path(dll_dir: Option<&Path>, data: Option<&Path>) -> Option<PathBuf> {
    let named = dll_dir
        .and_then(|d| std::fs::read_to_string(d.join("datadir.txt")).ok())
        .and_then(|t| t.trim_start_matches('\u{feff}').lines().nth(1).map(|l| l.trim().to_string()))
        .filter(|l| !l.is_empty())
        .map(PathBuf::from);
    named.or_else(|| data.map(|d| d.join("dict").join("words.sqlite"))).filter(|p| p.is_file())
}

/// settings.json changed (or the service started): read the options, the theme, and reopen the word files.
pub fn configure(dll_dir: Option<&Path>, dirs: &[PathBuf]) {
    let found = dirs.iter().find_map(|d| std::fs::read_to_string(d.join("settings.json")).ok().and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok()).map(|v| (d.clone(), v)));
    let (data, s) = match found {
        Some((d, v)) => (Some(d), v),
        None => (dirs.first().cloned(), Value::Null),
    };
    let str_of = |k: &str, d: &str| s.get(k).and_then(Value::as_str).unwrap_or(d).to_string();
    let color_mode = str_of("color", "system");
    let dark = color_mode == "dark" || (color_mode == "system" && windows_dark());
    let id = s.get("appearance.theme").and_then(Value::as_str).map(canonical).unwrap_or("default").to_string();
    let safe = id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-');
    let colors = BUILT_IN
        .iter()
        .find(|(i, _)| *i == id)
        .and_then(|(_, t)| theme_colors(t, dark))
        .or_else(|| data.as_ref().filter(|_| safe).and_then(|d| std::fs::read_to_string(d.join("themes").join(format!("{id}.json"))).ok()).and_then(|t| theme_colors(&t, dark)))
        .or_else(|| theme_colors(BUILT_IN[0].1, dark));
    let mut o = Opts::default();
    o.on = s.get("typing.suggestions").and_then(Value::as_bool).unwrap_or(true);
    o.count = s.get("typing.suggestion_count").and_then(Value::as_u64).unwrap_or(5).clamp(1, 9) as usize;
    o.prefer_dict = str_of("typing.preselect", "dict") != "plain";
    o.anchored = str_of("suggestions.position", "follow") == "anchored";
    // Hind Siliguri lives inside likhon.exe, not in Windows: the keyboard uses the chosen font, else Nirmala UI.
    if let Some(f) = s.get("appearance.font").and_then(Value::as_str).filter(|f| !f.is_empty() && f.len() <= 64) {
        o.face = f.to_string();
    }
    o.size = s.get("appearance.font_size").and_then(Value::as_f64).unwrap_or(17.0).clamp(13.0, 24.0) as f32 + 1.0;
    if let Some(c) = colors {
        o.colors = c;
    }
    let dict = dictionary_path(dll_dir, data.as_deref()).and_then(|p| Dictionary::open(&p).ok());
    // user.sqlite (user.db before 2026-10-24; likhon.exe renames it, never the keyboard).
    let user = data.as_ref().map(|d| if d.join("user.sqlite").is_file() { d.join("user.sqlite") } else { d.join("user.db") }).and_then(|p| UserData::open_read_only(&p).ok());
    LIST.with(|l| {
        if let Ok(mut l) = l.try_borrow_mut() {
            l.opts = o;
            l.dict = dict;
            l.user = user;
        }
    });
}

// ---- The window ----------------------------------------------------------------------------------------------

/// The service is active on this thread: make the (hidden) window.
pub fn open(pick: Rc<dyn Fn(String)>) {
    let hwnd = unsafe {
        let wc = WNDCLASSW { lpfnWndProc: Some(wnd_proc), hInstance: module(), lpszClassName: CLASS, hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(), ..Default::default() };
        RegisterClassW(&wc); // fails harmlessly when another thread of this app registered it
        CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, CLASS, PCWSTR::null(), WS_POPUP, 0, 0, 0, 0, None, None, Some(module()), None).ok()
    };
    LIST.with(|l| {
        if let Ok(mut l) = l.try_borrow_mut() {
            l.hwnd = hwnd;
            l.pick = Some(pick);
        }
    });
}

/// The service is deactivated: remove the window (and the class once no thread uses it, so an unloaded DLL leaves
/// nothing behind) and close the word files.
pub fn close() {
    let hwnd = LIST.with(|l| l.try_borrow_mut().ok().and_then(|mut l| std::mem::take(&mut *l).hwnd));
    unsafe {
        if let Some(h) = hwnd {
            let _ = DestroyWindow(h);
        }
        let _ = UnregisterClassW(CLASS, Some(module()));
    }
}

/// The word changed: work out the rows and show them by the word (`at`, screen coordinates; None: by the caret).
/// Nothing shows when suggestions are off, in a password field, or for text that is not Bangla.
pub fn show(keys: &str, text: &str, at: Option<RECT>, password: bool) {
    let bangla = text.chars().any(|c| ('\u{0980}'..='\u{09FF}').contains(&c));
    let shown = LIST.with(|l| {
        let mut l = l.try_borrow_mut().ok()?;
        if !l.opts.on || password || !bangla || text.is_empty() {
            l.items.clear();
            return None;
        }
        let limit = l.opts.count;
        let mine = l.user.as_ref().map(|u| u.words_starting_with(text, limit)).unwrap_or_default();
        let mut items = candidates(text, None, &mine, l.dict.as_ref(), limit);
        let mut sel = preselect(&items, text, l.dict.as_ref(), l.opts.prefer_dict);
        if let Some(choice) = l.user.as_ref().and_then(|u| u.get(UserList::Choices, keys)) {
            sel = choose(&mut items, choice, limit);
        }
        l.items = items;
        l.sel = sel;
        l.keys = keys.to_string();
        Some((l.hwnd?, l.items.len(), l.opts.anchored))
    });
    match shown {
        Some((hwnd, n, anchored)) => unsafe {
            place(hwnd, n, if anchored { None } else { at });
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = InvalidateRect(Some(hwnd), None, false);
        },
        None => hide(),
    }
}

/// Suggestions are showing.
pub fn showing() -> bool {
    LIST.with(|l| l.try_borrow().map(|l| !l.items.is_empty()).unwrap_or(false))
}

/// Up and Down.
pub fn step(down: bool) {
    let hwnd = LIST.with(|l| {
        let mut l = l.try_borrow_mut().ok()?;
        let n = l.items.len();
        if n == 0 {
            return None;
        }
        l.sel = if down { (l.sel + 1) % n } else { (l.sel + n - 1) % n };
        l.hwnd
    });
    if let Some(h) = hwnd {
        unsafe {
            let _ = InvalidateRect(Some(h), None, false);
        }
    }
}

/// The selected word.
pub fn selected() -> Option<String> {
    LIST.with(|l| l.try_borrow().ok().and_then(|l| l.items.get(l.sel).map(|c| c.word.clone())))
}

pub fn hide() {
    let hwnd = LIST.with(|l| {
        let mut l = l.try_borrow_mut().ok()?;
        l.items.clear();
        l.hwnd
    });
    if let Some(h) = hwnd {
        unsafe {
            let _ = ShowWindow(h, SW_HIDE);
        }
    }
}

/// A word was finished or picked: likhon.exe may learn it when it runs (its own password check applies too).
pub fn commit(word: &str) {
    let Some(agent) = (unsafe { FindWindowW(AGENT_CLASS, PCWSTR::null()).ok().filter(|h| !h.is_invalid()) }) else { return };
    let from = LIST.with(|l| l.try_borrow().ok().and_then(|l| l.hwnd)).unwrap_or_default();
    if word.is_empty() || from.is_invalid() {
        return;
    }
    let w: Vec<u16> = word.encode_utf16().collect();
    let cds = COPYDATASTRUCT { dwData: TAG_COMMIT, cbData: (w.len() * 2) as u32, lpData: w.as_ptr() as *mut _ };
    unsafe {
        // likhon.exe only queues it (app/src/tsfbridge.rs): this does not wait on its work.
        SendMessageTimeoutW(agent, WM_COPYDATA, WPARAM(from.0 as usize), LPARAM(&cds as *const _ as isize), SMTO_ABORTIFHUNG, 100, None);
    }
}

fn scale(hwnd: HWND) -> f32 {
    unsafe { GetDpiForWindow(hwnd).max(96) as f32 / 96.0 }
}

/// Beside the word (below it, or above near the bottom of the screen); anchored: under Likhon's top bar when it shows.
unsafe fn place(hwnd: HWND, rows: usize, at: Option<RECT>) {
    let k = scale(hwnd);
    let (w, h) = ((W * k).round() as i32, ((HEAD + PAD * 2.0 + ROW * rows as f32) * k).round() as i32);
    let gap = (4.0 * k) as i32;
    let bar = FindWindowW(w!("LikhonTopBar"), PCWSTR::null()).ok().filter(|b| !b.is_invalid() && IsWindowVisible(*b).as_bool());
    let mut r = RECT::default();
    let anchor = match (at, bar) {
        (Some(a), _) => Some(a),
        (None, Some(b)) if GetWindowRect(b, &mut r).is_ok() => Some(r),
        _ => caret_rect(),
    };
    let Some(a) = anchor else { return };
    let mon = MonitorFromPoint(POINT { x: a.left, y: a.bottom }, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
    let wa = if GetMonitorInfoW(mon, &mut mi).as_bool() { mi.rcWork } else { RECT { left: 0, top: 0, right: 1920, bottom: 1080 } };
    let mut y = a.bottom + gap;
    if y + h > wa.bottom {
        y = a.top - gap - h;
    }
    let x = a.left.clamp(wa.left, (wa.right - w).max(wa.left));
    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y.max(wa.top), w, h, SWP_NOACTIVATE);
}

/// The caret of this thread's focused window on screen (the keyboard runs in the app's own thread).
unsafe fn caret_rect() -> Option<RECT> {
    let mut gi = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
    GetGUIThreadInfo(windows::Win32::System::Threading::GetCurrentThreadId(), &mut gi).ok()?;
    if gi.hwndCaret.is_invalid() {
        return None;
    }
    let mut a = POINT { x: gi.rcCaret.left, y: gi.rcCaret.top };
    let mut b = POINT { x: gi.rcCaret.right, y: gi.rcCaret.bottom };
    let _ = ClientToScreen(gi.hwndCaret, &mut a);
    let _ = ClientToScreen(gi.hwndCaret, &mut b);
    Some(RECT { left: a.x, top: a.y, right: b.x.max(a.x + 1), bottom: b.y.max(a.y + 1) })
}

unsafe fn font(face: &str, px: i32, weight: i32) -> HFONT {
    let f = wide(face);
    CreateFontW(-px, 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET, OUT_TT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, (VARIABLE_PITCH.0 | FF_SWISS.0) as u32, PCWSTR(f.as_ptr()))
}

unsafe fn text_at(dc: HDC, s: &str, r: RECT, color: u32, f: HFONT) {
    let old = SelectObject(dc, f.into());
    SetTextColor(dc, rgb(color));
    let mut w: Vec<u16> = s.encode_utf16().collect();
    let mut r = r;
    DrawTextW(dc, &mut w, &mut r, DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
    SelectObject(dc, old);
}

unsafe fn paint(hwnd: HWND) {
    let mut ps = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut ps);
    let snapshot = LIST.with(|l| l.try_borrow().ok().map(|l| (l.opts.clone(), l.keys.clone(), l.items.iter().map(|c| c.word.clone()).collect::<Vec<_>>(), l.sel)));
    if let Some((o, keys, words, sel)) = snapshot {
        let k = scale(hwnd);
        let px = |v: f32| (v * k).round() as i32;
        let mut rc = RECT::default();
        let _ = GetClientRect(hwnd, &mut rc);
        let c = o.colors;
        // Drawn into a memory bitmap first, so the list does not flicker while the selection moves.
        let mem = CreateCompatibleDC(Some(dc));
        let bmp = CreateCompatibleBitmap(dc, rc.right, rc.bottom);
        let old_bmp = SelectObject(mem, bmp.into());
        let ground = CreateSolidBrush(rgb(c.surface));
        FillRect(mem, &rc, ground);
        let pen = CreatePen(PS_SOLID, 1, rgb(c.line));
        let old_pen = SelectObject(mem, pen.into());
        let old_brush = SelectObject(mem, GetStockObject(NULL_BRUSH));
        let _ = Rectangle(mem, 0, 0, rc.right, rc.bottom);
        SetBkMode(mem, TRANSPARENT);
        let head = font("Segoe UI", px(13.0), 400);
        let num = font("Segoe UI", px(12.0), 400);
        let word = font(&o.face, px(o.size), 400);
        text_at(mem, &keys, RECT { left: px(10.0), top: 0, right: rc.right - px(10.0), bottom: px(HEAD) }, c.muted, head);
        let sel_brush = CreateSolidBrush(rgb(c.sel));
        for (i, w) in words.iter().enumerate() {
            let top = px(HEAD + PAD + ROW * i as f32);
            let bottom = px(HEAD + PAD + ROW * (i + 1) as f32);
            if i == sel {
                let r = RECT { left: px(4.0), top, right: rc.right - px(4.0), bottom };
                FillRect(mem, &r, sel_brush);
            }
            text_at(mem, &(i + 1).to_string(), RECT { left: px(14.0), top, right: px(30.0), bottom }, c.muted, num);
            text_at(mem, w, RECT { left: px(34.0), top, right: rc.right - px(10.0), bottom }, c.ink, word);
        }
        let _ = BitBlt(dc, 0, 0, rc.right, rc.bottom, Some(mem), 0, 0, SRCCOPY);
        SelectObject(mem, old_brush);
        SelectObject(mem, old_pen);
        SelectObject(mem, old_bmp);
        for o in [HGDIOBJ::from(ground), pen.into(), head.into(), num.into(), word.into(), sel_brush.into(), bmp.into()] {
            let _ = DeleteObject(o);
        }
        let _ = DeleteDC(mem);
    }
    let _ = EndPaint(hwnd, &ps);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_PAINT => {
            unsafe { paint(hwnd) };
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_LBUTTONUP => {
            // A click on a row takes it (the service puts it in, in the focused document).
            let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32 / scale(hwnd);
            if y >= HEAD + PAD {
                let i = ((y - HEAD - PAD) / ROW) as usize;
                let picked = LIST.with(|l| l.try_borrow().ok().and_then(|l| Some((l.items.get(i)?.word.clone(), l.pick.clone()?))));
                if let Some((word, pick)) = picked {
                    pick(word);
                }
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
