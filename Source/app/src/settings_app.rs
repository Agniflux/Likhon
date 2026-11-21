//! Settings mode: `likhon.exe --settings [page]` (Desktop/CLAUDE.md section 1).
//!
//! A separate process with one window: a WebView2 web view (through wry and tao, the libraries Tauri 2
//! is built on) showing settings-ui/dist, which build.rs embeds in the exe. The page asks for things
//! through `call(cmd, args)` over the web view's IPC channel; this file answers. Changes are written to
//! settings.json and the agent is told to reload it, so they apply at once. If WebView2 is missing, a
//! plain message box explains it and typing keeps working (the agent is another process).
//!
//! The window has no Windows frame (v2 §7 draws its own 36 px title bar): the page asks for drag, resize, minimize,
//! maximize and close through IPC, and the window keeps the system shadow.
//!
//! Nothing here goes online: the page is served from memory under the `likhon` scheme, and navigation
//! anywhere else is refused. Links to the Likhon site (settings::site(), likhon.agniflux.com by default) open in the default browser, never in the web view.

use crate::settings;
use crate::text::{t, wide, Lang};
use serde_json::{json, Value};
use std::borrow::Cow;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::platform::windows::{IconExtWindows, WindowBuilderExtWindows};
use tao::window::{Icon, ResizeDirection, Theme, WindowBuilder};
use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use wry::http::{header::CONTENT_TYPE, Response};
use wry::{WebContext, WebViewBuilder, WebViewBuilderExtWindows};

include!(concat!(env!("OUT_DIR"), "/ui.rs"));

/// The Settings window's class, so a second `--settings` (or the agent) can find it.
pub const WINDOW_CLASS: &str = "LikhonSettingsWindow";

enum UserEvent {
    Eval(String),
    Title(String),
    Theme(Option<Theme>),
    Show,
    Window(String, String),
}

pub fn run(page: Option<String>) -> i32 {
    unsafe {
        // One Settings window: a second request brings the open one to the front.
        CreateMutexW(std::ptr::null(), 0, wide("Local\\Likhon.Settings").as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let hwnd = FindWindowW(wide(WINDOW_CLASS).as_ptr(), std::ptr::null());
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
            return 0;
        }
    }

    let s = settings::load();
    let lang = s.language;
    let page = page.unwrap_or_else(|| "general".into());

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window = match WindowBuilder::new()
        .with_title(t("app.set.window", lang, &[]))
        .with_inner_size(LogicalSize::new(960.0, 640.0))
        .with_min_inner_size(LogicalSize::new(760.0, 520.0))
        .with_decorations(false)
        .with_undecorated_shadow(true)
        .with_visible(false) // shown once the page has drawn, so there is no white flash
        .with_theme(theme_for(&s.color))
        .with_window_icon(Icon::from_resource(1, None).ok())
        .with_window_classname(WINDOW_CLASS)
        .with_drag_and_drop(false)
        .build(&event_loop)
    {
        Ok(w) => w,
        Err(_) => return missing(lang),
    };

    // WebView2 keeps its cache in a folder; keep it out of the portable folder.
    let mut context = WebContext::new(Some(std::env::temp_dir().join("Likhon").join("WebView2")));
    let dark = matches!(theme_for(&s.color), Some(Theme::Dark)) || (s.color == "system" && window.theme() == Theme::Dark);
    let builder = WebViewBuilder::new_with_web_context(&mut context)
        .with_custom_protocol("likhon".into(), move |_id, request| serve(request.uri().path(), &page))
        .with_url("likhon://localhost/index.html")
        .with_background_color(if dark { (0x16, 0x16, 0x18, 255) } else { (0xF6, 0xF6, 0xF4, 255) })
        .with_navigation_handler(|url| url.starts_with("http://likhon.localhost/") || url.starts_with("likhon://"))
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_devtools(cfg!(debug_assertions))
        .with_browser_accelerator_keys(false)
        .with_default_context_menus(cfg!(debug_assertions))
        .with_ipc_handler(move |request| handle(request.body(), &proxy));
    let webview = match builder.build(&window) {
        Ok(w) => w,
        Err(_) => return missing(lang),
    };

    let mut was_maximized = false;
    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UserEvent::Eval(js)) => {
                let _ = webview.evaluate_script(&js);
            }
            Event::UserEvent(UserEvent::Title(title)) => window.set_title(&title),
            Event::UserEvent(UserEvent::Theme(theme)) => window.set_theme(theme),
            Event::UserEvent(UserEvent::Show) => {
                window.set_visible(true);
                window.set_focus();
            }
            Event::UserEvent(UserEvent::Window(what, dir)) => match what.as_str() {
                "drag" => {
                    let _ = window.drag_window();
                }
                "resize" => {
                    let d = match dir.as_str() {
                        "n" => ResizeDirection::North,
                        "s" => ResizeDirection::South,
                        "e" => ResizeDirection::East,
                        "w" => ResizeDirection::West,
                        "ne" => ResizeDirection::NorthEast,
                        "nw" => ResizeDirection::NorthWest,
                        "se" => ResizeDirection::SouthEast,
                        _ => ResizeDirection::SouthWest,
                    };
                    if !window.is_maximized() {
                        let _ = window.drag_resize_window(d);
                    }
                }
                "minimize" => window.set_minimized(true),
                "maximize" => window.set_maximized(!window.is_maximized()),
                "close" => {
                    notify_agent(MSG_CAPTURE, 0);
                    *flow = ControlFlow::Exit;
                }
                _ => {}
            },
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                // The page draws the title bar, so it is told when the window is maximized or restored.
                let m = window.is_maximized();
                if m != was_maximized {
                    was_maximized = m;
                    let _ = webview.evaluate_script(&format!("window.__likhon.maximized({m})"));
                }
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                notify_agent(MSG_CAPTURE, 0); // never leave the agent paused
                *flow = ControlFlow::Exit;
            }
            _ => {}
        }
    })
}

fn theme_for(color: &str) -> Option<Theme> {
    match color {
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => None, // follow Windows
    }
}

/// WebView2 is missing (or failed): a plain message, the only notice that takes focus (Toasts design).
fn missing(lang: Lang) -> i32 {
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide(&t("app.set.missingBody", lang, &[])).as_ptr(),
            wide(&t("app.set.missingTitle", lang, &[])).as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
    1
}

// ---- Files ----------------------------------------------------------------------------------------

fn serve(path: &str, page: &str) -> Response<Cow<'static, [u8]>> {
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let (body, kind): (Cow<'static, [u8]>, &str) = if path == "boot.json" {
        (Cow::Owned(boot(page).to_string().into_bytes()), "application/json")
    } else if let Some((_, bytes)) = UI.iter().find(|(p, _)| *p == path) {
        let kind = match path.rsplit('.').next() {
            Some("html") => "text/html; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("js") => "text/javascript; charset=utf-8",
            Some("json") => "application/json",
            Some("woff2") => "font/woff2",
            Some("txt") => "text/plain; charset=utf-8",
            _ => "application/octet-stream",
        };
        (Cow::Borrowed(*bytes), kind)
    } else {
        return Response::builder().status(404).body(Cow::Borrowed(&b""[..])).unwrap();
    };
    Response::builder().header(CONTENT_TYPE, kind).body(body).unwrap()
}

fn boot(page: &str) -> Value {
    let (dir, portable) = settings::dir();
    let diag = dir.join(DIAG_FILE);
    json!({
        "settings": Value::Object(settings::stored()),
        "defaults": Value::Object(settings::defaults()),
        "page": page,
        "version": env!("CARGO_PKG_VERSION"),
        // The native add-on (installer, text service) arrives with M3; every build is Portable for now.
        "edition": "portable",
        "site": settings::site(),
        "startWithWindows": win::starts_with_windows(),
        // Likhon in Windows' keyboard list (likhon_tsf.dll registered machine-wide: winkb.rs, the installer or the scripts).
        "windowsKeyboard": win::tsf_registered(),
        // A newer keyboard is inside this copy than the one installed (Settings offers Update).
        "windowsKeyboardStale": win::tsf_registered() && crate::winkb::stale(),
        "learned": likhon_store::UserData::open(&likhon_store::user_db(&dir)).map(|u| u.learned_count()).unwrap_or(0),
        "windows": win::windows_version(),
        "dataDir": dir.to_string_lossy(),
        "dataPortable": portable,
        "dictionary": settings::dictionary_path().and_then(|p| likhon_store::Dictionary::open(&p).ok().map(|d| (p, d.info()))).map(|(p, i)| json!({
            "words": i.words, "path": p.to_string_lossy(), "license": i.license, "snapshot": i.snapshot,
        })),
        "diagnostics": if diag.exists() { Value::String(diag.to_string_lossy().into_owned()) } else { Value::Null },
    })
}

// ---- Requests from the page -------------------------------------------------------------------------

fn handle(body: &str, proxy: &EventLoopProxy<UserEvent>) {
    let Ok(req) = serde_json::from_str::<Value>(body) else { return };
    let id = req["id"].as_u64().unwrap_or(0);
    let args = &req["args"];
    let reply = match req["cmd"].as_str().unwrap_or("") {
        "ready" => {
            let _ = proxy.send_event(UserEvent::Show);
            Value::Null
        }
        "set" => {
            let key = args["key"].as_str().unwrap_or_default();
            if settings::defaults().contains_key(key) {
                settings::set(key, args["value"].clone());
                if key == "color" {
                    let _ = proxy.send_event(UserEvent::Theme(theme_for(args["value"].as_str().unwrap_or("system"))));
                }
                notify_agent(MSG_SETTINGS_CHANGED, 0);
            }
            Value::Null
        }
        "reset" => {
            let keys: Vec<String> = args["keys"].as_array().map(|a| a.iter().filter_map(|k| k.as_str().map(String::from)).collect()).unwrap_or_default();
            settings::update(|m| keys.iter().for_each(|k| {
                m.remove(k);
            }));
            let _ = proxy.send_event(UserEvent::Theme(theme_for(&settings::load().color)));
            notify_agent(MSG_SETTINGS_CHANGED, 0);
            Value::Null
        }
        "title" => {
            let _ = proxy.send_event(UserEvent::Title(args["text"].as_str().unwrap_or_default().to_string()));
            Value::Null
        }
        "openFolder" => {
            let _ = std::process::Command::new("explorer.exe").arg(settings::dir().0).spawn();
            Value::Null
        }
        "capture" => {
            notify_agent(MSG_CAPTURE, args["on"].as_bool().unwrap_or(false) as usize);
            Value::Null
        }
        "hotkeyFree" => Value::Bool(win::hotkey_free(args)),
        "runningApps" => win::running_apps(),
        "pickExe" => win::pick_exe(),
        "paste" => win::clipboard_text().map(Value::String).unwrap_or(Value::Null),
        "copy" => {
            win::set_clipboard_text(args["text"].as_str().unwrap_or_default());
            Value::Null
        }
        "phonetic" => {
            let options = settings::load().engine;
            Value::String(likhon_engine::type_keys(args["text"].as_str().unwrap_or_default(), options))
        }
        "win" => {
            let _ = proxy.send_event(UserEvent::Window(args["do"].as_str().unwrap_or_default().into(), args["dir"].as_str().unwrap_or_default().into()));
            Value::Null
        }
        "openUrl" => {
            win::open_site(args["url"].as_str().unwrap_or_default());
            Value::Null
        }
        "userList" | "userAdd" | "userRemove" | "userImport" | "userExport" => user_list(req["cmd"].as_str().unwrap_or_default(), args),
        "tool" => {
            // The text tools run here, in the engine (no text leaves the computer).
            use likhon_engine::tools;
            let text = args["text"].as_str().unwrap_or_default();
            if text.chars().count() > 20_000 {
                return reply(proxy, id, Value::Null);
            }
            match args["name"].as_str().unwrap_or_default() {
                "taka" => tools::taka(text).map(|(w, a)| json!({ "words": w, "amount": a })).unwrap_or(Value::Null),
                "clean" => Value::String(tools::clean(text)),
                "num" => Value::String(tools::numbers(text)),
                "how" => tools::how_to_type(text).map(Value::String).unwrap_or(Value::Null),
                // Legacy encoding (engine src/mpl, MPL-2.0 from Avro Keyboard): both directions.
                "toBijoy" => Value::String(likhon_engine::unicode_to_bijoy(text)),
                "fromBijoy" => Value::String(likhon_engine::bijoy_to_unicode(text)),
                // Proshika (Likhon's own map, engine src/proshika.tsv): both directions.
                "toProshika" => Value::String(likhon_engine::unicode_to_proshika(text)),
                "fromProshika" => Value::String(likhon_engine::proshika_to_unicode(text)),
                _ => Value::Null,
            }
        }
        "layoutChart" => {
            // A layout's keys for the layout viewer, straight from its JSON file: built in, else added (library or file).
            let id = args["id"].as_str().unwrap_or_default();
            crate::layouts::BUILT_IN.iter().find(|(i, _)| *i == id).and_then(|(_, t)| serde_json::from_str::<Value>(t).ok())
                .or_else(|| crate::sideload::list("layouts").into_iter().find(|(i, _)| i == id).map(|(_, v)| v))
                .map(|v| json!({ "keys": v["keys"], "name": v["name"], "author": v["author"] })).unwrap_or(Value::Null)
        }
        "sideloadAdd" => {
            // Layouts and themes only, JSON only, checked strictly; the page shows the not-reviewed warning first.
            let kind = args["kind"].as_str().unwrap_or_default();
            match win::open_json_file() {
                None => Value::Null,
                Some(text) => match crate::sideload::add(kind, &text) {
                    Ok((id, name)) => {
                        notify_agent(MSG_SETTINGS_CHANGED, 0);
                        json!({ "id": id, "name": name })
                    }
                    Err(e) => json!({ "error": e }),
                },
            }
        }
        "sideloadRemove" => Value::Bool(crate::sideload::remove(args["kind"].as_str().unwrap_or_default(), args["id"].as_str().unwrap_or_default())),
        "sideloaded" => {
            let kind = args["kind"].as_str().unwrap_or_default();
            let mut all: Vec<Value> = crate::sideload::list(kind).into_iter().map(|(_, v)| v).collect();
            if kind == "layouts" {
                // Input methods from the library (Khipro): no key chart, but chosen and removed like the others.
                all.extend(crate::sideload::input_methods().into_iter().map(|(id, en, bn)| json!({ "id": id, "name": { "en": en, "bn": bn }, "kind": "input-method" })));
            }
            Value::Array(all)
        }
        "openOsk" => {
            notify_agent(MSG_OSK, 0);
            Value::Null
        }
        // The site, only because the person clicked (online.rs); each runs on its own thread and replies when done.
        "checkUpdate" | "libraryList" | "libraryAdd" | "libraryRemove" | "dictDownload" | "donate" => {
            let cmd = req["cmd"].as_str().unwrap_or_default().to_string();
            let args = args.clone();
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                let v = match cmd.as_str() {
                    "checkUpdate" => crate::online::latest(),
                    "libraryList" => crate::online::library(),
                    "libraryAdd" => crate::online::add(&args["item"]),
                    "libraryRemove" => crate::online::remove(args["id"].as_str().unwrap_or_default()),
                    "dictDownload" => crate::online::dictionary(),
                    _ => crate::online::donate(&args["words"].as_array().map(|a| a.iter().filter_map(|w| w.as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default()),
                };
                reply(&proxy, id, v);
            });
            return;
        }
        "learnedWords" => {
            // The words Likhon learned while typing, for the donate review list (the person picks what is sent). Words
            // the dictionary already has are left out: only new words are worth donating.
            let dict = settings::dictionary_path().and_then(|p| likhon_store::Dictionary::open(&p).ok());
            let words = likhon_store::UserData::open(&likhon_store::user_db(&settings::dir().0)).map(|db| db.learned()).unwrap_or_default();
            Value::Array(words.into_iter().filter(|w| !dict.as_ref().is_some_and(|d| d.contains(w))).map(Value::String).collect())
        }
        "forgetLearned" => {
            if let Ok(db) = likhon_store::UserData::open(&likhon_store::user_db(&settings::dir().0)) {
                db.forget_learned();
            }
            notify_agent(MSG_USER_CHANGED, 0);
            Value::Null
        }
        "libraryAdded" => crate::online::added(),
        "windowsKeyboard" => {
            // Add or remove Likhon in Win+Space (winkb.rs): the elevated helper registers machine-wide, and this
            // process (not elevated) changes this user's keyboard list. Windows asks for permission; waits on its own thread.
            let on = args["on"].as_bool().unwrap_or(false);
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                if !on {
                    crate::winkb::user_list(false);
                }
                let data = settings::dir().0.to_string_lossy().replace('"', "");
                let r = crate::fontdefault::run_elevated(&if on { format!("--keyboard-add \"{data}\"") } else { "--keyboard-remove".into() });
                match r {
                    Ok(0) if on => {
                        crate::winkb::user_list(true);
                    }
                    Err(_) if !on => {
                        crate::winkb::user_list(true); // cancelled: put it back in the list
                    }
                    _ => {}
                }
                reply(&proxy, id, match r {
                    Ok(0) => json!({ "ok": true, "added": win::tsf_registered() }),
                    Ok(code) => json!({ "ok": false, "code": code, "noFiles": code == crate::winkb::NO_FILES as u32, "added": win::tsf_registered() }),
                    Err(e) => json!({ "ok": false, "error": e, "added": win::tsf_registered() }),
                });
            });
            return;
        }
        "openWindowsKeyboards" => {
            win::open_windows_keyboards();
            Value::Null
        }
        "fontDefaultNow" => crate::fontdefault::current().map(Value::String).unwrap_or(Value::Null),
        "fontDefault" => {
            // Restore only: the font changer was removed from Settings (owner decision 2026-10-24), and Restore undoes
            // what an earlier copy changed. Elevated helper (fontdefault.rs); waits on its own thread.
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                let r = crate::fontdefault::run_elevated("--font-default-restore");
                reply(&proxy, id, match r {
                    Ok(0) => json!({ "ok": true, "face": crate::fontdefault::current() }),
                    Ok(code) => json!({ "ok": false, "code": code }),
                    Err(e) => json!({ "ok": false, "error": e }),
                });
            });
            return;
        }
        "startWithWindows" => Value::Bool(win::set_start_with_windows(args["on"].as_bool().unwrap_or(false))),
        "bnFonts" => Value::Array(bangla_fonts().into_iter().map(Value::String).collect()),
        "diagPreview" => Value::String(diagnostics()),
        "diagMake" => {
            let p = settings::dir().0.join(DIAG_FILE);
            let _ = std::fs::write(&p, diagnostics());
            Value::String(p.to_string_lossy().into_owned())
        }
        _ => Value::Null,
    };
    let _ = proxy.send_event(UserEvent::Eval(format!("window.__likhon.reply({id}, {reply})")));
}

fn reply(proxy: &EventLoopProxy<UserEvent>, id: u64, v: Value) {
    let _ = proxy.send_event(UserEvent::Eval(format!("window.__likhon.reply({id}, {v})")));
}

// ---- Telling the agent ----------------------------------------------------------------------------

/// Messages to the agent's hidden window (agent.rs). WM_APP numbers, the same in both modes.
pub const MSG_SETTINGS_CHANGED: u32 = WM_APP + 10;
pub const MSG_OPEN_TOPBAR: u32 = WM_APP + 11;
/// wParam 1: the Settings window is picking a key, so the agent leaves the keyboard alone; 0: done.
pub const MSG_CAPTURE: u32 = WM_APP + 12;
/// user.sqlite changed (My words, autocorrect, shortcuts): the agent reloads its lists.
pub const MSG_USER_CHANGED: u32 = WM_APP + 13;
/// Open (or close) the on-screen keyboard.
pub const MSG_OSK: u32 = WM_APP + 14;
/// The hook finished a word that may be learned (agent.rs takes it from its queue).
pub const MSG_LEARN: u32 = WM_APP + 15;
/// A quick hotkey (quick.rs); wParam is its key.
pub const MSG_QUICK: u32 = WM_APP + 16;
pub const AGENT_CLASS: &str = "LikhonAgentWindow";

pub fn notify_agent(msg: u32, wparam: usize) {
    unsafe {
        let hwnd: HWND = FindWindowW(wide(AGENT_CLASS).as_ptr(), std::ptr::null());
        if !hwnd.is_null() {
            PostMessageW(hwnd, msg, wparam, 0 as LPARAM);
        }
    }
}

// ---- Diagnostics (S13) ------------------------------------------------------------------------------

const DIAG_FILE: &str = "likhon-diagnostics.txt";

/// What S13 promises, and nothing more: no typed text, no words, no app names, no personal names.
fn diagnostics() -> String {
    let s = settings::load();
    let stored = settings::stored();
    let defaults = settings::defaults();
    let mut changed: Vec<&str> = stored.keys().filter(|k| defaults.get(*k) != stored.get(*k)).map(String::as_str).collect();
    changed.sort();
    format!(
        "Likhon diagnostics\n\nLikhon: {} (portable)\nEngine: {}\nWindows: {}\nDisplay scale: {}%\nLayout: {}\nOutput: {}\nTheme: {}\nSettings changed from the defaults: {}\nRecent errors: none recorded\n",
        env!("CARGO_PKG_VERSION"),
        likhon_engine::VERSION,
        win::windows_version(),
        win::scale_percent(),
        s.layout,
        s.output,
        s.theme.as_deref().unwrap_or("default"),
        if changed.is_empty() { "none".to_string() } else { changed.join(", ") },
    )
}

// ---- My words, autocorrect, shortcuts (user.sqlite) ------------------------------------------------------

/// user.sqlite in the data folder (Desktop/CLAUDE.md 7). The agent is told after each change, so typing uses it at once.
fn user_list(cmd: &str, args: &Value) -> Value {
    let Some(list) = args["list"].as_str().and_then(likhon_store::List::parse) else { return Value::Null };
    let Ok(db) = likhon_store::UserData::open(&likhon_store::user_db(&settings::dir().0)) else { return json!({ "error": "open" }) };
    let changed = |v: Value| {
        notify_agent(MSG_USER_CHANGED, 0);
        v
    };
    match cmd {
        "userList" => Value::Array(db.all(list).into_iter().map(|e| json!({ "key": e.key, "value": e.value, "added": e.added })).collect()),
        "userAdd" => match db.add(list, args["key"].as_str().unwrap_or_default(), args["value"].as_str().unwrap_or_default()) {
            Ok(()) => changed(json!({ "ok": true })),
            Err(e) => json!({ "error": e }),
        },
        "userRemove" => changed(Value::Bool(db.remove(list, args["key"].as_str().unwrap_or_default()))),
        "userImport" => match win::open_text_file() {
            Some(text) => {
                let (added, skipped) = db.import(list, &text);
                changed(json!({ "added": added, "skipped": skipped }))
            }
            None => Value::Null,
        },
        "userExport" => {
            let name = format!("likhon-{}.txt", args["list"].as_str().unwrap_or("list"));
            Value::Bool(win::save_text_file(&name, &db.export(list)))
        }
        _ => Value::Null,
    }
}

// ---- Fonts (archive S16) ----------------------------------------------------------------------------

/// Installed font families that have Bangla letters (they map ক, U+0995), sorted, by their English name.
fn bangla_fonts() -> Vec<String> {
    use windows::core::w;
    use windows::Win32::Graphics::DirectWrite::*;
    let mut out = Vec::new();
    unsafe {
        let Ok(dw) = DWriteCreateFactory::<IDWriteFactory>(DWRITE_FACTORY_TYPE_SHARED) else { return out };
        let mut fc: Option<IDWriteFontCollection> = None;
        if dw.GetSystemFontCollection(&mut fc, false).is_err() {
            return out;
        }
        let Some(fc) = fc else { return out };
        for i in 0..fc.GetFontFamilyCount() {
            let Ok(fam) = fc.GetFontFamily(i) else { continue };
            let Ok(font) = fam.GetFirstMatchingFont(DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL) else { continue };
            if !font.HasCharacter(0x0995).is_ok_and(|b| b.as_bool()) {
                continue;
            }
            let Ok(names) = fam.GetFamilyNames() else { continue };
            let (mut idx, mut found) = (0u32, windows::core::BOOL(0));
            if names.FindLocaleName(w!("en-us"), &mut idx, &mut found).is_err() || !found.as_bool() {
                idx = 0;
            }
            let len = names.GetStringLength(idx).unwrap_or(0) as usize;
            let mut buf = vec![0u16; len + 1];
            if names.GetString(idx, &mut buf).is_ok() {
                out.push(String::from_utf16_lossy(&buf[..len]));
            }
        }
    }
    out.sort_by_key(|n| n.to_lowercase());
    out.dedup();
    out
}

// ---- Win32 helpers ----------------------------------------------------------------------------------

pub(crate) mod win {
    use crate::text::wide;
    use serde_json::{json, Value};
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
    use windows_sys::Win32::System::DataExchange::*;
    use windows_sys::Win32::System::Memory::*;
    use windows_sys::Win32::System::Registry::*;
    use windows_sys::Win32::System::Threading::*;
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    const CF_UNICODETEXT: u32 = 13;

    /// Whether another program already holds this key combination (RegisterHotKey fails if so).
    pub fn hotkey_free(k: &Value) -> bool {
        let vk = k["vk"].as_u64().unwrap_or(0) as u32;
        let mods = (if k["ctrl"].as_bool() == Some(true) { MOD_CONTROL } else { 0 })
            | (if k["alt"].as_bool() == Some(true) { MOD_ALT } else { 0 })
            | (if k["shift"].as_bool() == Some(true) { MOD_SHIFT } else { 0 });
        if vk == VK_F12 as u32 && mods == 0 {
            return true; // Windows keeps plain F12 for debuggers, so it cannot be tested; the agent reads it from its hook
        }
        unsafe {
            if RegisterHotKey(std::ptr::null_mut(), 0xB0F0, mods | MOD_NOREPEAT, vk) != 0 {
                UnregisterHotKey(std::ptr::null_mut(), 0xB0F0);
                true
            } else {
                false
            }
        }
    }

    fn exe_of_window(hwnd: HWND) -> Option<String> {
        unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == 0 || pid == GetCurrentProcessId() {
                return None;
            }
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
            CloseHandle(h);
            (ok != 0).then(|| String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    /// The program's own name from its version information ("Microsoft Word"), else the file name.
    fn describe(path: &str) -> (String, String) {
        let exe = path.rsplit(['\\', '/']).next().unwrap_or(path).to_string();
        let fallback = exe.trim_end_matches(".exe").trim_end_matches(".EXE").to_string();
        unsafe {
            let wpath = wide(path);
            let size = GetFileVersionInfoSizeW(wpath.as_ptr(), std::ptr::null_mut());
            if size == 0 {
                return (fallback, exe);
            }
            let mut data = vec![0u8; size as usize];
            if GetFileVersionInfoW(wpath.as_ptr(), 0, size, data.as_mut_ptr().cast()) == 0 {
                return (fallback, exe);
            }
            let mut tr: *mut u16 = std::ptr::null_mut();
            let mut tr_len = 0u32;
            let mut lang = "040904b0".to_string();
            if VerQueryValueW(data.as_ptr().cast(), wide("\\VarFileInfo\\Translation").as_ptr(), (&mut tr as *mut *mut u16).cast(), &mut tr_len) != 0 && tr_len >= 4 {
                lang = format!("{:04x}{:04x}", *tr, *tr.add(1));
            }
            for field in ["FileDescription", "ProductName"] {
                let mut p: *mut u16 = std::ptr::null_mut();
                let mut n = 0u32;
                let key = wide(&format!("\\StringFileInfo\\{lang}\\{field}"));
                if VerQueryValueW(data.as_ptr().cast(), key.as_ptr(), (&mut p as *mut *mut u16).cast(), &mut n) != 0 && n > 1 {
                    let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n as usize - 1)).trim().to_string();
                    if !s.is_empty() {
                        return (s, exe);
                    }
                }
            }
            (fallback, exe)
        }
    }

    /// Programs with a visible window now, for "Add app" (App rules). Only names, nothing from inside them.
    pub fn running_apps() -> Value {
        unsafe extern "system" fn each(hwnd: HWND, list: LPARAM) -> windows_sys::core::BOOL {
            let list = &mut *(list as *mut Vec<String>);
            if IsWindowVisible(hwnd) != 0 && GetWindow(hwnd, GW_OWNER).is_null() && GetWindowTextLengthW(hwnd) > 0 && GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW == 0 {
                if let Some(p) = exe_of_window(hwnd) {
                    if !list.iter().any(|x| x.eq_ignore_ascii_case(&p)) {
                        list.push(p);
                    }
                }
            }
            1
        }
        let mut paths: Vec<String> = Vec::new();
        unsafe { EnumWindows(Some(each), &mut paths as *mut _ as LPARAM) };
        let mut apps: Vec<(String, String)> = paths.iter().map(|p| describe(p)).filter(|(_, e)| !e.eq_ignore_ascii_case("explorer.exe")).collect();
        apps.sort_by_key(|(n, _)| n.to_lowercase());
        Value::Array(apps.into_iter().map(|(name, exe)| json!({ "name": name, "exe": exe })).collect())
    }

    /// "Choose another program from a fileâ€¦": a standard Open dialog for .exe files.
    pub fn pick_exe() -> Value {
        unsafe {
            let mut file = [0u16; 1024];
            let filter: Vec<u16> = "Programs (*.exe)\0*.exe\0\0".encode_utf16().collect();
            let mut ofn: OPENFILENAMEW = std::mem::zeroed();
            ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.hwndOwner = GetForegroundWindow();
            ofn.lpstrFilter = filter.as_ptr();
            ofn.lpstrFile = file.as_mut_ptr();
            ofn.nMaxFile = file.len() as u32;
            ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
            if GetOpenFileNameW(&mut ofn) == 0 {
                return Value::Null;
            }
            let len = file.iter().position(|&c| c == 0).unwrap_or(file.len());
            let (name, exe) = describe(&String::from_utf16_lossy(&file[..len]));
            json!({ "name": name, "exe": exe })
        }
    }

    // .dat: word choice lists kept by other keyboards are plain text with that ending.
    const TEXT_FILTER: &str = "Text files (*.txt, *.dat)\0*.txt;*.dat\0All files (*.*)\0*.*\0\0";
    /// The largest list file read (50,000 entries of a few words each fit easily).
    const MAX_TEXT_FILE: u64 = 8 * 1024 * 1024;

    /// Asks for a layout or theme file (.json), read as UTF-8, at most 64 KB.
    pub fn open_json_file() -> Option<String> {
        unsafe {
            let mut file = [0u16; 1024];
            let filter: Vec<u16> = "Likhon layout or theme (*.json)\0*.json\0\0".encode_utf16().collect();
            let mut ofn: OPENFILENAMEW = std::mem::zeroed();
            ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.hwndOwner = GetForegroundWindow();
            ofn.lpstrFilter = filter.as_ptr();
            ofn.lpstrFile = file.as_mut_ptr();
            ofn.nMaxFile = file.len() as u32;
            ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
            if GetOpenFileNameW(&mut ofn) == 0 {
                return None;
            }
            let len = file.iter().position(|&c| c == 0).unwrap_or(file.len());
            let path = String::from_utf16_lossy(&file[..len]);
            if std::fs::metadata(&path).ok()?.len() > 64 * 1024 {
                return Some(String::from("{\"too\": \"large\"}"));
            }
            std::fs::read_to_string(path).ok().map(|t| t.trim_start_matches('\u{feff}').to_string())
        }
    }

    /// Asks for a text file and reads it as UTF-8 (a UTF-16 file from Notepad is read too).
    pub fn open_text_file() -> Option<String> {
        unsafe {
            let mut file = [0u16; 1024];
            let filter: Vec<u16> = TEXT_FILTER.encode_utf16().collect();
            let mut ofn: OPENFILENAMEW = std::mem::zeroed();
            ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.hwndOwner = GetForegroundWindow();
            ofn.lpstrFilter = filter.as_ptr();
            ofn.lpstrFile = file.as_mut_ptr();
            ofn.nMaxFile = file.len() as u32;
            ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
            if GetOpenFileNameW(&mut ofn) == 0 {
                return None;
            }
            let len = file.iter().position(|&c| c == 0).unwrap_or(file.len());
            let path = String::from_utf16_lossy(&file[..len]);
            if std::fs::metadata(&path).ok()?.len() > MAX_TEXT_FILE {
                return None;
            }
            let bytes = std::fs::read(path).ok()?;
            if bytes.starts_with(&[0xFF, 0xFE]) {
                let u: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                return Some(String::from_utf16_lossy(&u));
            }
            Some(String::from_utf8_lossy(&bytes).into_owned())
        }
    }

    /// Asks where to save, then writes `text` as UTF-8.
    pub fn save_text_file(name: &str, text: &str) -> bool {
        unsafe {
            let mut file = [0u16; 1024];
            for (i, u) in name.encode_utf16().take(1000).enumerate() {
                file[i] = u;
            }
            let filter: Vec<u16> = TEXT_FILTER.encode_utf16().collect();
            let ext: Vec<u16> = "txt\0".encode_utf16().collect();
            let mut ofn: OPENFILENAMEW = std::mem::zeroed();
            ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.hwndOwner = GetForegroundWindow();
            ofn.lpstrFilter = filter.as_ptr();
            ofn.lpstrFile = file.as_mut_ptr();
            ofn.nMaxFile = file.len() as u32;
            ofn.lpstrDefExt = ext.as_ptr();
            ofn.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
            if GetSaveFileNameW(&mut ofn) == 0 {
                return false;
            }
            let len = file.iter().position(|&c| c == 0).unwrap_or(file.len());
            std::fs::write(String::from_utf16_lossy(&file[..len]), text).is_ok()
        }
    }

    pub fn clipboard_text() -> Option<String> {
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return None;
            }
            let h = GetClipboardData(CF_UNICODETEXT);
            let out = if h.is_null() {
                None
            } else {
                let p = GlobalLock(h as HGLOBAL) as *const u16;
                let s = if p.is_null() {
                    None
                } else {
                    let mut n = 0;
                    while *p.add(n) != 0 {
                        n += 1;
                    }
                    Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, n)))
                };
                GlobalUnlock(h as HGLOBAL);
                s
            };
            CloseClipboard();
            out
        }
    }

    pub fn set_clipboard_text(text: &str) {
        let w = wide(text);
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return;
            }
            EmptyClipboard();
            let h = GlobalAlloc(GMEM_MOVEABLE, w.len() * 2);
            if !h.is_null() {
                let p = GlobalLock(h) as *mut u16;
                if !p.is_null() {
                    std::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
                    GlobalUnlock(h);
                    if SetClipboardData(CF_UNICODETEXT, h as HANDLE).is_null() {
                        GlobalFree(h);
                    }
                }
            }
            CloseClipboard();
        }
    }

    fn reg_string(name: &str) -> Option<String> {
        unsafe {
            let mut buf = [0u16; 256];
            let mut size = (buf.len() * 2) as u32;
            let ok = RegGetValueW(
                HKEY_LOCAL_MACHINE,
                wide("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion").as_ptr(),
                wide(name).as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buf.as_mut_ptr().cast(),
                &mut size,
            );
            (ok == ERROR_SUCCESS).then(|| String::from_utf16_lossy(&buf[..(size as usize / 2).saturating_sub(1)]))
        }
    }

    /// Opens a page of the Likhon site in the default browser. Only https addresses on the site (net::on_site) made
    /// of plain path characters are allowed, so the page can never start another program or reach another site.
    pub fn open_site(url: &str) {
        if url.len() > 200 || !crate::net::on_site(url) || !url["https://".len()..].bytes().all(|b| b.is_ascii_alphanumeric() || b"/-_.?=&:".contains(&b)) {
            return;
        }
        unsafe {
            windows_sys::Win32::UI::Shell::ShellExecuteW(std::ptr::null_mut(), wide("open").as_ptr(), wide(url).as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
        }
    }

    /// Whether the Likhon text service is registered (its COM class in HKLM).
    pub fn tsf_registered() -> bool {
        use windows_sys::Win32::System::Registry::*;
        let path = wide(r"Software\Classes\CLSID\{C772DC88-B930-41B1-A45A-C800A07FAD7C}\InprocServer32");
        let mut k = std::ptr::null_mut();
        let ok = unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.as_ptr(), 0, KEY_READ, &mut k) } == 0;
        if ok {
            unsafe { RegCloseKey(k) };
        }
        ok
    }

    /// Windows Settings › Time & language › Language & region, where keyboards are added.
    pub fn open_windows_keyboards() {
        unsafe {
            windows_sys::Win32::UI::Shell::ShellExecuteW(std::ptr::null_mut(), wide("open").as_ptr(), wide("ms-settings:regionlanguage").as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
        }
    }

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    /// Whether this Windows user's Run entry starts this likhon.exe (the copy that is running now).
    pub fn starts_with_windows() -> bool {
        use windows_sys::Win32::System::Registry::*;
        let mut buf = vec![0u16; 1024];
        let mut size = (buf.len() * 2) as u32;
        let ok = unsafe { RegGetValueW(HKEY_CURRENT_USER, wide(RUN_KEY).as_ptr(), wide("Likhon").as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut size) } == 0;
        let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
        ok && String::from_utf16_lossy(&buf[..(size as usize / 2).saturating_sub(1)]).to_lowercase().contains(&exe)
    }

    /// Start with Windows, for this Windows user only (no administrator): a Run entry for this likhon.exe.
    pub fn set_start_with_windows(on: bool) -> bool {
        use windows_sys::Win32::System::Registry::*;
        unsafe {
            let mut key = std::ptr::null_mut();
            if RegCreateKeyExW(HKEY_CURRENT_USER, wide(RUN_KEY).as_ptr(), 0, std::ptr::null(), 0, KEY_SET_VALUE, std::ptr::null(), &mut key, std::ptr::null_mut()) != 0 {
                return false;
            }
            let ok = if on {
                let v = wide(&format!("\"{}\"", std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()));
                RegSetValueExW(key, wide("Likhon").as_ptr(), 0, REG_SZ, v.as_ptr().cast(), (v.len() * 2) as u32) == 0
            } else {
                let r = RegDeleteValueW(key, wide("Likhon").as_ptr());
                r == 0 || r == 2
            };
            RegCloseKey(key);
            ok
        }
    }

    pub fn windows_version() -> String {
        let build = reg_string("CurrentBuild").unwrap_or_default();
        let name = if build.parse::<u32>().unwrap_or(0) >= 22000 { "Windows 11" } else { "Windows 10" };
        format!("{name} {} (build {build})", reg_string("DisplayVersion").unwrap_or_default())
    }

    pub fn scale_percent() -> u32 {
        unsafe { GetDpiForSystem() * 100 / 96 }
    }
}
