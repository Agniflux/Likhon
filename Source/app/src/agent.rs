//! Agent mode: Portable typing through a low-level keyboard hook (Desktop/CLAUDE.md, section 5), the tray
//! icon, and the top bar (topbar.rs).
//!
//! Safety (root CLAUDE.md, "Dev safety"):
//! - Starts in English unless "Start in Bangla" is on: the toggle key (F12 by default) switches.
//! - Ctrl+Alt+F12 exits at once. `scripts/kill-likhon.ps1` stops it from outside.
//! - Our own injected input is ignored, so it can never feed itself.
//! - A mouse click, another window, or any non-letter key ends the current word, so the engine never
//!   erases text it did not type.
//! - Nothing typed is stored, logged, or sent. Only timing counts are kept (see `Stats`).
//!
//! Still to come (later milestones):
//! - The hook callback runs the engine and calls SendInput itself. The brief prefers handing keys to an
//!   engine thread; measure first (SPIKES.md), change if the numbers say so.
//! - Keys map to characters as on a US keyboard. Real layouts come with the layout files (M5/M6).

use crate::settings::{self, Settings};
use crate::settings_app::{AGENT_CLASS, MSG_CAPTURE, MSG_OPEN_TOPBAR, MSG_OSK, MSG_SETTINGS_CHANGED, MSG_USER_CHANGED};
use crate::words::{Lists, Word};
use crate::text::{t, wide};
use crate::topbar;
use likhon_engine::fixed::{Fixed, KeyPress};
use likhon_engine::mit::khipro::Khipro;
use likhon_engine::{Action, Phonetic};
use std::cell::RefCell;
use std::time::Instant;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::System::Threading::{CreateMutexW, GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows_sys::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

/// Marks input we inject, so the hook can ignore it ("LKHN").
pub(crate) const MAGIC: usize = 0x4C4B_484E;
const WM_TRAY: u32 = WM_APP + 1;
/// Posted from the hook when the mode changes, so the tray and top bar update outside the hook callback.
const WM_REFRESH: u32 = WM_APP + 2;
/// Typing switched: show the caret badge (wParam 1 Bangla, 0 English).
const WM_BADGE: u32 = WM_APP + 3;
/// The next-layout key was pressed.
const WM_NEXT_LAYOUT: u32 = WM_APP + 4;
/// Shift+F12 (legacy output on): flip between Bijoy and Unicode.
const WM_TOGGLE_OUTPUT: u32 = WM_APP + 5;
const TIMER_WATCHDOG: usize = 1;
/// Win+Space changes the keyboard without moving focus: checked this often (windows_keyboard).
const TIMER_KEYBOARD: usize = 2;
/// Resource IDs in likhon.rc: the app icon, then one tray icon per state, light taskbar then dark (v2 §2).
const ICON_APP: u16 = 1;
const TRAY_ICONS: u16 = 10;

/// What the tray icon shows (v2 Desktop §2).
#[derive(Clone, Copy, PartialEq)]
enum TrayState {
    Bangla,
    English,
    /// The focused window belongs to a program Likhon cannot type into (one running as administrator).
    Paused,
    /// The keyboard hook could not be installed: typing does not work.
    Error,
}
/// Reinstall the hooks after this long without any input event (Windows can drop a slow hook silently).
const WATCHDOG_IDLE_MS: u128 = 10_000;

struct State {
    bangla: bool,
    settings: Settings,
    engine: Phonetic,
    /// The fixed layout in use, or None for phonetic (settings: layout).
    fixed: Option<Fixed>,
    /// Khipro (layout "khipro"): typed by character, like phonetic, from the Khipro team's own rules.
    khipro: Option<Khipro>,
    /// The last word finished, while the caret has not moved since: what the next word follows.
    next_from: Option<String>,
    /// The word being typed (for shortcuts and autocorrect when it ends) and the person's lists.
    word: Word,
    /// Legacy (Bijoy) output: what the word is in Unicode and what was typed for it on screen (settings: output).
    legacy: crate::legacy::Legacy,
    lists: Lists,
    hwnd: HWND,
    kb_hook: HHOOK,
    mouse_hook: HHOOK,
    last_window: HWND,
    last_event: Instant,
    /// Keys whose key-down we swallowed: swallow their key-up too.
    swallowed: [bool; 256],
    /// The Settings window is picking a new key: leave the keyboard alone.
    capturing: bool,
    stats: Stats,
    taskbar_created: u32,
    /// The focused window is out of reach (TrayState::Paused).
    paused: bool,
    /// The focused app types with a Bangla Windows keyboard (the Likhon keyboard or another): Bangla only, the top
    /// bar cannot switch to English there (owner decision 2026-10-24).
    windows_keyboard: bool,
    /// Foreground-change listener (updates `paused`).
    fg_hook: HWINEVENTHOOK,
}

/// Words finished in the hook, waiting for the agent thread to check the field and learn them. Never written anywhere
/// else, never logged; dropped when the field may be a password field.
static LEARN_QUEUE: std::sync::Mutex<Vec<(Option<String>, String)>> = std::sync::Mutex::new(Vec::new());

impl State {
    /// A word was finished with Space, Enter or Tab, or picked from the suggestions: offer it for learning (with the
    /// word before it, for next-word suggestions), and show the words that usually follow it.
    fn offer_learn(&mut self, word: &str) {
        let bangla_word = self.bangla && word.chars().any(|c| ('\u{0980}'..='\u{09FF}').contains(&c));
        let prev = std::mem::take(&mut self.next_from);
        if !bangla_word {
            return;
        }
        if self.settings.learn {
            if let Ok(mut q) = LEARN_QUEUE.lock() {
                if q.len() < 64 {
                    q.push((prev, word.to_string()));
                }
            }
            unsafe { PostMessageW(self.hwnd, crate::settings_app::MSG_LEARN, 0, 0) };
        }
        self.next_from = Some(word.to_string());
        if self.settings.next_word && self.settings.suggestions {
            unsafe { PostMessageW(self.hwnd, crate::suggest::WM_SUGGEST, 0, 0) };
        }
    }

    /// The current word ends (the caret may have moved): both typing paths start fresh.
    fn reset_typing(&mut self) {
        // The caret may have moved: the next word does not follow the last one.
        self.next_from = None;
        if !self.word.is_empty() {
            unsafe { PostMessageW(self.hwnd, crate::suggest::WM_SUGGEST, 0, 0) }; // hides the suggestion window
        }
        self.word.clear();
        self.legacy.clear();
        self.engine.reset();
        if let Some(k) = self.khipro.as_mut() {
            k.reset();
        }
        if let Some(f) = self.fixed.as_mut() {
            f.reset();
        }
    }
}

/// Timing only, never which keys (root CLAUDE.md: never log what a person types).
#[derive(Default)]
struct Stats {
    keys_handled: u64,
    micros: Vec<u32>,
    hook_reinstalls: u32,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|s| s.try_borrow_mut().ok()?.as_mut().map(f))
}

// ---- Used by the top bar --------------------------------------------------------------------------

/// Bangla is typed in the focused app: by the hook when switched on, or always by a Bangla Windows keyboard.
pub fn bangla() -> bool {
    with(|s| s.bangla || s.windows_keyboard).unwrap_or(false)
}

/// The focused app types with a Bangla Windows keyboard: Bangla only, nothing to switch.
pub fn windows_keyboard() -> bool {
    with(|s| s.windows_keyboard).unwrap_or(false)
}

/// Checks the focused app's keyboard (focus moved, or the timer: Win+Space moves no focus) and redraws on a change.
/// While a Bangla Windows keyboard types, the taskbar shows that keyboard's own icon (the Likhon logo for the Likhon
/// keyboard), so the tray icon steps aside: one icon, not two (owner, 2026-10-24).
unsafe fn check_windows_keyboard() {
    let now = foreground_bangla_keyboard();
    if with(|s| std::mem::replace(&mut s.windows_keyboard, now) != now).unwrap_or(false) {
        tray(if now { NIM_DELETE } else { NIM_ADD });
        topbar::refresh();
    }
}

/// The focused window belongs to a program Likhon cannot type into (top bar and tray show "paused").
pub fn paused() -> bool {
    with(|s| s.paused).unwrap_or(false)
}

/// The keyboard hook is not installed: typing does not work (top bar and tray show "error").
pub fn hook_failed() -> bool {
    with(|s| s.kb_hook.is_null()).unwrap_or(false)
}

/// "Click to restart" on the error state: install the hooks again.
pub fn restart_hooks() {
    unsafe { install_hooks() };
    topbar::refresh();
}

pub fn settings() -> Settings {
    with(|s| s.settings.clone()).unwrap_or_default()
}

pub fn set_bangla(on: bool) {
    with(|s| {
        s.bangla = on;
        s.reset_typing();
    });
    unsafe { tray(NIM_MODIFY) };
    topbar::refresh();
    crate::osk::refresh();
}

/// The top bar was closed: it lives on in the tray. The first time, say so (Desktop/CLAUDE.md 6).
pub fn topbar_closed() {
    settings::set("topbar.hidden", true.into());
    let first = with(|s| {
        s.settings.topbar_hidden = true;
        !std::mem::replace(&mut s.settings.tray_notice_shown, true)
    })
    .unwrap_or(false);
    if first {
        settings::set("tray_notice_shown", true.into());
        unsafe { balloon("app.topbar.trayTitle", "app.topbar.trayBody") };
    }
}

/// Open the Settings window (a separate process; typing never depends on it).
/// The on-screen keyboard and Letters panel type here (into the focused app; our window never takes focus).
pub fn type_text(text: &str) {
    with(|s| {
        s.reset_typing();
        unsafe { emit(s, 0, text) };
        s.legacy.clear();
    });
}

/// The on-screen keyboard's Backspace.
pub fn press_backspace() {
    with(|s| s.reset_typing());
    unsafe { send(1, "") };
}

/// A click on a suggestion row: replace the word with it.
pub fn commit_word(w: &str) {
    with(|s| {
        if !s.word.is_empty() {
            unsafe { emit(s, s.word.len(), w) };
            s.reset_typing();
            s.offer_learn(w);
        } else if s.next_from.is_some() {
            // A next word, between words: typed with its space, and the list moves on to what follows it.
            unsafe { emit(s, 0, &format!("{w} ")) };
            s.reset_typing();
            s.offer_learn(w);
            return;
        }
        crate::suggest::hide();
    });
}

/// The pin in the suggestion window: follow the text cursor, or stay anchored.
pub fn toggle_anchored() {
    let anchored = with(|s| s.settings.suggest_anchored).unwrap_or(false);
    settings::set("suggestions.position", if anchored { "follow" } else { "anchored" }.into());
    reload_settings();
    unsafe { PostMessageW(with(|s| s.hwnd).unwrap_or(std::ptr::null_mut()), crate::suggest::WM_SUGGEST, 0, 0) };
}

/// The top bar's layout picker: switch the layout and apply it at once (Settings sees it in settings.json).
/// Shift+F12 or the top bar's output pill, while legacy output is switched on: Bijoy to Unicode and back.
pub fn toggle_output() {
    let Some((enabled, output)) = with(|s| (s.settings.legacy_enabled, s.settings.output.clone())) else { return };
    if !enabled {
        return;
    }
    settings::set("output", if output == "bijoy" { "unicode" } else { "bijoy" }.into());
    reload_settings();
}

pub fn set_layout(id: &str) {
    settings::set("layout", id.into());
    reload_settings();
}

pub fn open_settings(page: Option<&str>) {
    if let Ok(exe) = std::env::current_exe() {
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("--settings");
        if let Some(p) = page {
            cmd.arg(p);
        }
        let _ = cmd.spawn();
    }
}

// ---- Agent ---------------------------------------------------------------------------------------

pub fn run() -> i32 {
    unsafe {
        // One agent per user session. A second launch opens the top bar of the running one.
        CreateMutexW(std::ptr::null(), 0, wide("Local\\Likhon.Agent").as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            crate::settings_app::notify_agent(MSG_OPEN_TOPBAR, 0);
            return 0;
        }
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        let hinst = GetModuleHandleW(std::ptr::null());
        let class = wide(AGENT_CLASS);
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: LoadIconW(hinst, ICON_APP as _),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(0, class.as_ptr(), class.as_ptr(), 0, 0, 0, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(), hinst, std::ptr::null());
        if hwnd.is_null() {
            return 1;
        }
        crate::tsfbridge::allow(hwnd);

        let settings = settings::load();
        topbar::set_face(settings.font.as_deref());
        // v2 §6: the splash covers the start when a person starts Likhon by hand. Start with Windows (native add-on,
        // not built yet) will start the agent without it.
        crate::splash::show(settings.language);
        let taskbar_created = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
        let show_hint = !settings.tray_hint_shown;
        STATE.with(|s| {
            *s.borrow_mut() = Some(State {
                bangla: settings.start_bangla,
                engine: Phonetic::with_options(settings.engine),
                fixed: crate::layouts::load(&settings.layout).map(Fixed::new),
                khipro: crate::layouts::input_method(&settings.layout).map(Khipro::new),
                next_from: None,
                word: Word::default(),
                legacy: Default::default(),
                lists: Lists::load(),
                settings,
                hwnd,
                kb_hook: std::ptr::null_mut(),
                mouse_hook: std::ptr::null_mut(),
                last_window: std::ptr::null_mut(),
                last_event: Instant::now(),
                swallowed: [false; 256],
                capturing: false,
                stats: Stats::default(),
                taskbar_created,
                paused: false,
                windows_keyboard: false,
                fg_hook: std::ptr::null_mut(),
            })
        });
        install_hooks();
        // Event-driven, out of context: Windows calls us on this thread when the focused window changes.
        let fg_hook = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, std::ptr::null_mut(), Some(foreground_changed), 0, 0, WINEVENT_OUTOFCONTEXT);
        with(|s| s.fg_hook = fg_hook);
        foreground_changed(std::ptr::null_mut(), 0, std::ptr::null_mut(), 0, 0, 0, 0);
        tray(NIM_ADD);
        if show_hint {
            // First run: explain the toggle key and how to keep the icon visible (Desktop/CLAUDE.md section 6).
            balloon("app.hint.title", "app.hint.body");
            settings::set("tray_hint_shown", true.into());
            with(|s| s.settings.tray_hint_shown = true);
        }
        topbar::start();
        crate::suggest::start();
        SetTimer(hwnd, TIMER_WATCHDOG, 3000, None);
        SetTimer(hwnd, TIMER_KEYBOARD, 300, None);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        crate::suggest::stop();
        topbar::stop();
        tray(NIM_DELETE);
        with(|s| {
            UnhookWindowsHookEx(s.kb_hook);
            UnhookWindowsHookEx(s.mouse_hook);
            UnhookWinEvent(s.fg_hook);
            write_stats(&s.stats);
        });
        0
    }
}

unsafe fn install_hooks() {
    let hinst = GetModuleHandleW(std::ptr::null());
    with(|s| {
        if !s.kb_hook.is_null() {
            UnhookWindowsHookEx(s.kb_hook);
        }
        if !s.mouse_hook.is_null() {
            UnhookWindowsHookEx(s.mouse_hook);
        }
        s.kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hinst, 0);
        s.mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hinst, 0);
        s.last_event = Instant::now();
    });
    if !with(|s| s.hwnd.is_null()).unwrap_or(true) {
        tray(NIM_MODIFY);
    }
}

/// settings.json changed (the Settings window says so): apply it at once.
fn reload_settings() {
    let fresh = settings::load();
    topbar::set_face(fresh.font.as_deref());
    with(|s| {
        s.engine.set_options(fresh.engine);
        if fresh.legacy_encoding != s.settings.legacy_encoding || fresh.output != s.settings.output {
            s.legacy.clear(); // the word on screen is in the old encoding: start the next one fresh
        }
        if fresh.layout != s.settings.layout {
            s.fixed = crate::layouts::load(&fresh.layout).map(Fixed::new);
            s.khipro = crate::layouts::input_method(&fresh.layout).map(Khipro::new);
        }
        s.settings = fresh;
    });
    unsafe { tray(NIM_MODIFY) };
    topbar::refresh();
    // The on-screen keyboard shows the layout in use: a change from the top bar, Settings or the next-layout key.
    crate::osk::refresh();
}

fn tray_state() -> TrayState {
    with(|s| {
        if s.kb_hook.is_null() {
            TrayState::Error
        } else if s.paused {
            TrayState::Paused
        } else if s.bangla || s.windows_keyboard {
            TrayState::Bangla
        } else {
            TrayState::English
        }
    })
    .unwrap_or(TrayState::English)
}

/// Whether the taskbar uses the light theme (Settings > Personalization > Colors, "Windows mode").
fn taskbar_is_light() -> bool {
    let mut v = 0u32;
    let mut size = 4u32;
    let ok = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize").as_ptr(),
            wide("SystemUsesLightTheme").as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut v as *mut u32).cast(),
            &mut size,
        )
    };
    ok == 0 && v == 1
}

/// The tray icon for a state, sized for the current display scale and drawn for the taskbar's theme.
unsafe fn tray_icon(state: TrayState) -> HICON {
    let hinst = GetModuleHandleW(std::ptr::null());
    let size = GetSystemMetrics(SM_CXSMICON);
    let light = taskbar_is_light();
    // A chosen theme colours the Bangla and English icons too (paused and error keep their warning colours).
    if matches!(state, TrayState::Bangla | TrayState::English) {
        if let Some(icon) = topbar::themed_tray_icon(state == TrayState::Bangla, light, size) {
            THEMED_ICON.with(|c| {
                let old = c.replace(icon);
                if !old.is_null() {
                    DestroyIcon(old);
                }
            });
            return icon;
        }
    }
    let id = TRAY_ICONS + 2 * state as u16 + if light { 0 } else { 1 };
    LoadImageW(hinst, id as usize as _, IMAGE_ICON, size, size, LR_DEFAULTCOLOR) as HICON
}

thread_local! {
    /// The last themed tray icon made, destroyed when the next one replaces it.
    static THEMED_ICON: std::cell::Cell<HICON> = const { std::cell::Cell::new(std::ptr::null_mut()) };
}

/// The focused window changed: is it one we cannot type into? A program running as administrator does not let a
/// normal process query its token (and blocks our input anyway, UIPI), so a refused token means out of reach.
unsafe extern "system" fn foreground_changed(_: HWINEVENTHOOK, _: u32, _: HWND, _: i32, _: i32, _: u32, _: u32) {
    let mut pid = 0u32;
    GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
    let mut paused = false;
    if pid != 0 && pid != GetCurrentProcessId() {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if !h.is_null() {
            let mut token = std::ptr::null_mut();
            if OpenProcessToken(h, TOKEN_QUERY, &mut token) == 0 {
                paused = true;
            } else {
                let mut elev = TOKEN_ELEVATION { TokenIsElevated: 0 };
                let mut len = 0u32;
                if GetTokenInformation(token, TokenElevation, (&mut elev as *mut TOKEN_ELEVATION).cast(), std::mem::size_of::<TOKEN_ELEVATION>() as u32, &mut len) != 0 {
                    paused = elev.TokenIsElevated != 0 && !self_elevated();
                }
                CloseHandle(token);
            }
            CloseHandle(h);
        }
    }
    let changed = with(|s| std::mem::replace(&mut s.paused, paused) != paused).unwrap_or(false);
    if changed {
        tray(NIM_MODIFY);
        topbar::refresh();
    }
    check_windows_keyboard();
}

fn self_elevated() -> bool {
    unsafe {
        let mut token = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elev = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut len = 0u32;
        let ok = GetTokenInformation(token, TokenElevation, (&mut elev as *mut TOKEN_ELEVATION).cast(), std::mem::size_of::<TOKEN_ELEVATION>() as u32, &mut len);
        CloseHandle(token);
        ok != 0 && elev.TokenIsElevated != 0
    }
}

fn base_nid(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid
}

/// Compares a NUL-terminated wide string from Windows with `s`.
unsafe fn wide_eq(p: *const u16, s: &str) -> bool {
    let mut i = 0;
    for u in s.encode_utf16() {
        if *p.add(i) != u {
            return false;
        }
        i += 1;
    }
    *p.add(i) == 0
}

fn copy_into(dst: &mut [u16], s: &str) {
    for (i, u) in s.encode_utf16().take(dst.len() - 1).enumerate() {
        dst[i] = u;
    }
}

/// Tray icon: the state's icon (v2 §2) and the state in the tooltip.
unsafe fn tray(op: u32) {
    let Some((hwnd, lang, key, hidden)) = with(|s| (s.hwnd, s.settings.language, s.settings.toggle_label.clone(), s.windows_keyboard)) else { return };
    if hidden && op != NIM_DELETE {
        return; // a Bangla Windows keyboard shows its own icon now (check_windows_keyboard)
    }
    let state = tray_state();
    let mut nid = base_nid(hwnd);
    if op != NIM_DELETE {
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = tray_icon(state);
        let tip = match state {
            TrayState::Bangla => "app.tray.tipBangla",
            TrayState::English => "app.tray.tipEnglish",
            TrayState::Paused => "app.tray.tipPaused",
            TrayState::Error => "app.tray.tipError",
        };
        copy_into(&mut nid.szTip, &t(tip, lang, &[("key", &key)]));
    }
    Shell_NotifyIconW(op, &nid);
}

/// A one-time tray notification ("Likhon is running", "Still running in the tray").
unsafe fn balloon(title: &str, body: &str) {
    let Some((hwnd, lang, key)) = with(|s| (s.hwnd, s.settings.language, s.settings.toggle_label.clone())) else { return };
    let mut nid = base_nid(hwnd);
    nid.uFlags = NIF_INFO;
    nid.dwInfoFlags = NIIF_USER | NIIF_LARGE_ICON;
    nid.hBalloonIcon = tray_icon(TrayState::Bangla);
    copy_into(&mut nid.szInfoTitle, &t(title, lang, &[]));
    copy_into(&mut nid.szInfo, &t(body, lang, &[("key", &key)]));
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

/// A tray notification from phrases, with values filled in (quick hotkeys).
pub(crate) fn notice(title: &str, body: &str, vars: &[(&str, &str)]) {
    let Some((hwnd, lang)) = with(|s| (s.hwnd, s.settings.language)) else { return };
    unsafe {
        let mut nid = base_nid(hwnd);
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = NIIF_USER | NIIF_LARGE_ICON;
        nid.hBalloonIcon = tray_icon(TrayState::Bangla);
        copy_into(&mut nid.szInfoTitle, &t(title, lang, &[]));
        copy_into(&mut nid.szInfo, &t(body, lang, vars));
        Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

/// Right-click menu, the three items from Desktop/CLAUDE.md section 6, drawn like the top bar's menus (v2 §2). It never
/// takes the focus; a click elsewhere closes it (the mouse hook, topbar::mouse_down).
unsafe fn menu(_hwnd: HWND) {
    let mut pt = POINT { x: 0, y: 0 };
    GetCursorPos(&mut pt);
    topbar::open_tray_menu(pt);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER if wparam == TIMER_KEYBOARD => {
            check_windows_keyboard();
            0
        }
        WM_TIMER if wparam == TIMER_WATCHDOG => {
            // Watchdog: Windows removes a low-level hook that is too slow, without telling us.
            let idle = with(|s| s.last_event.elapsed().as_millis()).unwrap_or(0);
            if idle > WATCHDOG_IDLE_MS {
                install_hooks();
                with(|s| s.stats.hook_reinstalls += 1);
            }
            0
        }
        WM_SETTINGCHANGE => {
            if !(lparam as *const u16).is_null() && wide_eq(lparam as *const u16, "ImmersiveColorSet") {
                tray(NIM_MODIFY);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_TOGGLE_OUTPUT => {
            toggle_output();
            topbar::wake();
            0
        }
        WM_NEXT_LAYOUT => {
            // Phonetic, the built-in fixed layouts, then the added ones, round and round.
            let added = crate::layouts::added();
            let ids: Vec<&str> = std::iter::once("phonetic").chain(crate::layouts::BUILT_IN.iter().map(|(id, _)| *id)).chain(added.iter().map(|a| a.0.as_str())).collect();
            let cur = with(|s| s.settings.layout.clone()).unwrap_or_default();
            let next = ids.iter().position(|i| *i == cur).map(|i| ids[(i + 1) % ids.len()]).unwrap_or("phonetic");
            set_layout(next);
            topbar::wake();
            0
        }
        WM_BADGE => {
            let on = with(|s| s.settings.caret_badge).unwrap_or(false);
            crate::suggest::badge(wparam != 0, on);
            0
        }
        WM_REFRESH => {
            tray(NIM_MODIFY);
            topbar::refresh();
            0
        }
        WM_COPYDATA => {
            // The Likhon keyboard asks for suggestions (tsfbridge.rs).
            crate::tsfbridge::copydata(hwnd, wparam, lparam)
        }
        crate::tsfbridge::MSG_BRIDGE => {
            crate::tsfbridge::process(hwnd);
            0
        }
        crate::settings_app::MSG_LEARN => {
            // Out of the hook: learn only when the field is confirmed not to be a password field (unsure: not learned).
            let words = LEARN_QUEUE.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
            if !words.is_empty() && crate::suggest::password_field() == Some(false) {
                crate::suggest::learn(&words);
            }
            0
        }
        crate::settings_app::MSG_QUICK => {
            crate::quick::run(wparam as u16);
            0
        }
        MSG_OSK => {
            crate::osk::toggle();
            0
        }
        MSG_USER_CHANGED => {
            let lists = Lists::load();
            with(|s| s.lists = lists);
            crate::suggest::reload_user();
            0
        }
        crate::suggest::WM_SUGGEST => {
            // The word changed: work out the suggestions here, outside the keyboard hook.
            let now = with(|s| (s.word.keys().to_string(), s.word.text(), s.lists.replacement(&s.word, false, s.settings.autocorrect).map(String::from), s.settings.clone(), s.bangla, s.next_from.clone()));
            if let Some((keys, text, auto, settings, bangla, next_from)) = now {
                if bangla && !keys.is_empty() {
                    crate::suggest::update(&keys, &text, auto.as_deref(), &settings, crate::suggest::password_field() == Some(true));
                } else if let (true, true, Some(prev)) = (bangla, settings.next_word, next_from) {
                    // Between words: the words that usually follow the last one (none in a password field).
                    crate::suggest::show_next(&prev, &settings, crate::suggest::password_field() != Some(false));
                } else {
                    crate::suggest::hide();
                }
            }
            0
        }
        MSG_SETTINGS_CHANGED => {
            reload_settings();
            0
        }
        MSG_OPEN_TOPBAR => {
            topbar::open();
            0
        }
        MSG_CAPTURE => {
            with(|s| {
                s.capturing = wparam != 0;
                s.reset_typing();
            });
            0
        }
        WM_TRAY => {
            // Classic notify-icon messages: lParam is the mouse message.
            match (lparam as u32) & 0xFFFF {
                WM_RBUTTONUP | WM_CONTEXTMENU => menu(hwnd),
                WM_LBUTTONDBLCLK => topbar::open(),
                _ => {}
            }
            0
        }
        _ => {
            if with(|s| s.taskbar_created == msg).unwrap_or(false) {
                tray(NIM_ADD); // Explorer restarted
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let m = wparam as u32;
        let click = m == WM_LBUTTONDOWN || m == WM_RBUTTONDOWN || m == WM_MBUTTONDOWN;
        with(|s| {
            if click {
                s.reset_typing();
            }
            s.last_event = Instant::now();
        });
        if click {
            topbar::mouse_down((*(lparam as *const MSLLHOOKSTRUCT)).pt); // closes an open picker
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

fn down(vk: u16) -> bool {
    unsafe { GetKeyState(vk as i32) < 0 }
}

/// The lower-case file name of the program that owns the foreground window (App rules).
/// Only the name is read; nothing about what is typed there.
/// Whether the focused app types with a Bangla Windows keyboard: Likhon's own text service (likhon_tsf.dll, which
/// then does the typing and the toggle key itself) or another Bangla keyboard such as INSCRIPT. The hook stands aside
/// there, so text is never converted twice (Desktop/CLAUDE.md 2, hook and text service handoff).
unsafe fn foreground_bangla_keyboard() -> bool {
    let tid = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
    let hkl = windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout(tid) as usize;
    hkl & 0x3FF == 0x45 // LANG_BENGALI, any country
}

unsafe fn foreground_exe() -> Option<String> {
    let mut pid = 0u32;
    GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
    let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if h.is_null() {
        return None;
    }
    let mut buf = [0u16; 512];
    let mut len = buf.len() as u32;
    let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
    CloseHandle(h);
    if ok == 0 {
        return None;
    }
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    path.rsplit('\\').next().map(|n| n.to_ascii_lowercase())
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code != HC_ACTION as i32 {
        return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
    }
    let k = &*(lparam as *const KBDLLHOOKSTRUCT);
    if k.dwExtraInfo == MAGIC {
        return CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam);
    }
    let started = Instant::now();
    let vk = k.vkCode as u16;
    let msg = wparam as u32;
    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let injected_by_others = k.flags & LLKHF_INJECTED != 0;

    let swallow = with(|s| {
        s.last_event = Instant::now();
        let slot = (vk & 0xFF) as usize;
        if !is_down {
            // Swallow the key-up of a key whose key-down we swallowed.
            return std::mem::replace(&mut s.swallowed[slot], false);
        }
        let ctrl = down(VK_CONTROL);
        let alt = down(VK_MENU);
        let shift = down(VK_SHIFT);
        if vk == VK_F12 && ctrl && alt {
            PostQuitMessage(0); // kill switch, always on
            s.swallowed[slot] = true;
            return true;
        }
        if s.capturing || injected_by_others {
            s.reset_typing();
            return false;
        }

        if let Some(nk) = s.settings.next_layout {
            if vk == nk.vk && ctrl == nk.ctrl && alt == nk.alt && shift == nk.shift {
                s.reset_typing();
                PostMessageW(s.hwnd, WM_NEXT_LAYOUT, 0, 0);
                s.swallowed[slot] = true;
                return true;
            }
        }
        // Quick hotkeys (quick.rs): Ctrl + left Alt, so AltGr still types a layout's own level.
        if ctrl && down(VK_LMENU) && !down(VK_RMENU) && !shift && crate::quick::KEYS.contains(&vk) {
            s.reset_typing();
            PostMessageW(s.hwnd, crate::settings_app::MSG_QUICK, vk as usize, 0);
            s.swallowed[slot] = true;
            return true;
        }
        if vk == 0x4B && ctrl && alt && !shift {
            PostMessageW(s.hwnd, MSG_OSK, 0, 0);
            s.swallowed[slot] = true;
            return true;
        }
        if foreground_bangla_keyboard() {
            s.reset_typing();
            return false;
        }
        // Shift+F12 with legacy output on: Bijoy or Unicode (the same apps as F12 keep the key for themselves).
        if vk == VK_F12 && shift && !ctrl && !alt && s.settings.legacy_enabled && s.settings.toggle.vk == VK_F12 && !s.settings.toggle.shift {
            if foreground_exe().is_some_and(|exe| s.settings.passthrough.iter().any(|p| *p == exe)) {
                return false;
            }
            s.reset_typing();
            PostMessageW(s.hwnd, WM_TOGGLE_OUTPUT, 0, 0);
            s.swallowed[slot] = true;
            return true;
        }
        let tk = s.settings.toggle;
        if vk == tk.vk && ctrl == tk.ctrl && alt == tk.alt && shift == tk.shift {
            // App rules: these programs get the key for their own use (S06).
            if foreground_exe().is_some_and(|exe| s.settings.passthrough.iter().any(|p| *p == exe)) {
                s.reset_typing();
                return false;
            }
            s.bangla = !s.bangla;
            s.reset_typing();
            PostMessageW(s.hwnd, WM_REFRESH, 0, 0);
            PostMessageW(s.hwnd, WM_BADGE, s.bangla as usize, 0);
            s.swallowed[slot] = true;
            return true;
        }
        if !s.bangla {
            return false;
        }

        // A different window: whatever we typed before is not ours to edit any more.
        let fg = GetForegroundWindow();
        if fg != s.last_window {
            s.last_window = fg;
            s.reset_typing();
        }
        if matches!(vk, VK_SHIFT | VK_LSHIFT | VK_RSHIFT | VK_CAPITAL) {
            return false; // modifiers do not end a word
        }
        // The suggestion list is open: Up and Down move, Escape closes, Space and Enter take the selected word.
        if crate::suggest::visible() && !ctrl && !alt {
            match vk {
                VK_UP | VK_DOWN => {
                    crate::suggest::step(vk == VK_DOWN);
                    s.swallowed[slot] = true;
                    return true;
                }
                VK_ESCAPE => {
                    crate::suggest::hide();
                    s.swallowed[slot] = true;
                    return true;
                }
                // Number keys 1 to 9 take that row (Typing › Number keys), only while a word is being typed.
                0x31..=0x39 if !shift && s.settings.number_pick && s.word.len() > 0 => {
                    if let Some(w) = crate::suggest::word_at((vk - 0x31) as usize) {
                        emit(s, s.word.len(), &(w.clone() + " "));
                        s.reset_typing();
                        s.offer_learn(&w);
                        s.swallowed[slot] = true;
                        return true;
                    }
                }
                VK_SPACE | VK_RETURN if !shift => {
                    if let Some(w) = crate::suggest::selected_change(&s.word.text()) {
                        emit(s, s.word.len(), &if vk == VK_SPACE { w.clone() + " " } else { w.clone() });
                        s.reset_typing();
                        s.offer_learn(&w);
                        if vk == VK_RETURN {
                            s.next_from = None;
                            crate::suggest::hide();
                        }
                        s.swallowed[slot] = true;
                        return true;
                    }
                }
                _ => {}
            }
        }
        // The word ends: a shortcut expands (Tab, or Space when allowed), autocorrect fixes it (Space). The key that ended
        // it is typed after the replacement, in the same input, so it cannot arrive first.
        if (vk == VK_SPACE || vk == VK_TAB) && !shift && !ctrl && !alt {
            let shortcuts = vk == VK_TAB || s.settings.shortcuts_space;
            let autocorrect = vk == VK_SPACE && s.settings.autocorrect;
            if let Some(text) = s.lists.replacement(&s.word, shortcuts, autocorrect).map(String::from) {
                emit(s, s.word.len(), &if vk == VK_SPACE { text.clone() + " " } else { text.clone() });
                s.reset_typing();
                s.offer_learn(&text);
                s.swallowed[slot] = true;
                return true;
            }
        }
        // Emoji by name: the colon that closes ":name" turns the word into the emoji (engine emoji/, CLDR names).
        if s.settings.emoji && to_char(vk, shift, false) == Some(':') && !ctrl && !alt {
            let name = s.word.keys().strip_prefix(':').map(String::from);
            if let Some(e) = name.as_deref().and_then(likhon_engine::emoji::by_name) {
                emit(s, s.word.len(), e);
                s.reset_typing();
                s.swallowed[slot] = true;
                return true;
            }
        }
        // AltGr (right Alt, which Windows also reports as Ctrl) is a level of a fixed layout, not a shortcut.
        let altgr = s.fixed.is_some() && down(VK_RMENU);
        if ((ctrl || alt) && !altgr) || down(VK_LWIN) || down(VK_RWIN) {
            s.reset_typing(); // shortcuts go to the app untouched
            return false;
        }
        let action = if let Some(f) = s.fixed.as_mut() {
            // Fixed layouts type by key position: the key's unshifted US character names it.
            match to_char(vk, false, false).filter(|c| *c != ' ') {
                Some(key) => f.key(KeyPress { key, shift, altgr }),
                None => {
                    f.reset(); // Space, Backspace, Enter, arrows, function keys...
                    if matches!(vk, VK_SPACE | VK_RETURN | VK_TAB) {
                        let w = s.word.text();
                        s.offer_learn(&w);
                        s.word.clear();
                    }
                    return false;
                }
            }
        } else {
            let Some(ch) = to_char(vk, shift, GetKeyState(VK_CAPITAL as i32) & 1 == 1) else {
                if vk == VK_RETURN {
                    let w = s.word.text();
                    s.offer_learn(&w);
                }
                s.reset_typing(); // Backspace, Enter, arrows, function keys... (also ends the next-word chain)
                crate::suggest::hide();
                return false;
            };
            match s.khipro.as_mut() {
                Some(k) => k.key(ch),
                None => s.engine.key(ch),
            }
        };
        // Remember the word as typed (Space ends it), for shortcuts and autocorrect.
        match to_char(vk, shift, GetKeyState(VK_CAPITAL as i32) & 1 == 1) {
            Some(c) if !c.is_whitespace() => s.word.record(c, &action),
            Some(_) => {
                let w = s.word.text();
                s.offer_learn(&w);
                s.word.clear();
            }
            _ => s.word.clear(),
        }
        PostMessageW(s.hwnd, crate::suggest::WM_SUGGEST, 0, 0);
        match action {
            Action::PassThrough => {
                s.legacy.clear(); // a space or other key the app types itself ends the legacy word too
                false
            }
            Action::Replace { backspaces, output } => {
                emit(s, backspaces, &output);
                s.swallowed[slot] = true;
                s.stats.keys_handled += 1;
                if s.stats.micros.len() < 100_000 {
                    s.stats.micros.push(started.elapsed().as_micros().min(u32::MAX as u128) as u32);
                }
                true
            }
        }
    })
    .unwrap_or(false);

    if swallow {
        1
    } else {
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }
}

/// Types the engine's output: as it is (Unicode), or converted to Bijoy when legacy output is on. `backspaces` counts
/// the Unicode text being replaced; in Bijoy the word is converted whole and only the part that changed is retyped.
unsafe fn emit(s: &mut State, backspaces: usize, text: &str) {
    if s.settings.output == "bijoy" {
        let (erase, typed) = s.legacy.replace(backspaces, text, &s.settings.legacy_encoding);
        send(erase, &typed);
    } else {
        send(backspaces, text);
    }
}

/// Erase `backspaces` characters, then type `text` as Unicode input.
unsafe fn send(backspaces: usize, text: &str) {
    let mut inputs: Vec<INPUT> = Vec::with_capacity(backspaces * 2 + text.len() * 2);
    let key = |vk: u16, scan: u16, flags: u32| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: MAGIC } },
    };
    for _ in 0..backspaces {
        inputs.push(key(VK_BACK, 0, 0));
        inputs.push(key(VK_BACK, 0, KEYEVENTF_KEYUP));
    }
    for u in text.encode_utf16() {
        inputs.push(key(0, u, KEYEVENTF_UNICODE));
        inputs.push(key(0, u, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
    }
    if !inputs.is_empty() {
        SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

/// The character a key gives on a US keyboard (spike; real layouts come from layout files).
fn to_char(vk: u16, shift: bool, caps: bool) -> Option<char> {
    let c = match vk {
        0x41..=0x5A => {
            let lower = (b'a' + (vk - 0x41) as u8) as char;
            return Some(if shift ^ caps { lower.to_ascii_uppercase() } else { lower });
        }
        0x30..=0x39 => {
            let i = (vk - 0x30) as usize;
            return Some(if shift { b")!@#$%^&*("[i] as char } else { (b'0' + i as u8) as char });
        }
        VK_OEM_1 => (';', ':'),
        VK_OEM_PLUS => ('=', '+'),
        VK_OEM_COMMA => (',', '<'),
        VK_OEM_MINUS => ('-', '_'),
        VK_OEM_PERIOD => ('.', '>'),
        VK_OEM_2 => ('/', '?'),
        VK_OEM_3 => ('`', '~'),
        VK_OEM_4 => ('[', '{'),
        VK_OEM_5 => ('\\', '|'),
        VK_OEM_6 => (']', '}'),
        VK_OEM_7 => ('\'', '"'),
        VK_SPACE => (' ', ' '),
        _ => return None,
    };
    Some(if shift { c.1 } else { c.0 })
}

/// Timing figures for SPIKES.md, next to settings.json. No keys, no text.
fn write_stats(stats: &Stats) {
    let dir = settings::dir().0;
    let mut m = stats.micros.clone();
    m.sort_unstable();
    let pct = |p: f64| m.get(((m.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(0);
    let body = format!(
        "Likhon Spike A timing (handling time in the hook: engine + SendInput, microseconds)\nconverted keys: {}\nmedian: {}\np95: {}\np99: {}\nmax: {}\nhook reinstalls by the watchdog: {}\n",
        stats.keys_handled, pct(0.5), pct(0.95), pct(0.99), m.last().copied().unwrap_or(0), stats.hook_reinstalls
    );
    let _ = std::fs::write(dir.join("likhon-spike-a-timing.txt"), body);
}

#[cfg(test)]
mod tests {
    use super::to_char;

    #[test]
    fn keys_map_like_a_us_keyboard() {
        assert_eq!(to_char(0x41, false, false), Some('a'));
        assert_eq!(to_char(0x41, true, false), Some('A'));
        assert_eq!(to_char(0x41, true, true), Some('a'));
        assert_eq!(to_char(0x4F, true, false), Some('O'));
        assert_eq!(to_char(0x34, true, false), Some('$'));
        assert_eq!(to_char(0x31, false, false), Some('1'));
        assert_eq!(to_char(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_OEM_3, false, false), Some('`'));
        assert_eq!(to_char(windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_BACK, false, false), None);
    }
}
