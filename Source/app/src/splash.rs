//! The splash (Reference/design-v2/Likhon Desktop.dc.html §6): 480 × 320, frameless, never takes focus, shown for
//! the moment Likhon takes to start when a person starts it by hand. The artwork (the `splash-art` slot) is bundled
//! at build time as assets/splash/splash.jpg (scripts/icons/make-splash.ps1), never fetched. The name, tagline and
//! version sit on a solid cream strip, so their contrast never depends on the artwork.
//!
//! It holds for SHOW_MS, then fades out over FADE_MS (the design's only motion, §1), and a click closes it at once.

use crate::text::{t, wide, Lang};
use crate::tokens;
use crate::topbar::{rgb, text_layout, Gfx, Painter};
use std::cell::RefCell;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Imaging::*;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::{BeginPaint, EndPaint, GetMonitorInfoW, MonitorFromPoint, UpdateWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY, PAINTSTRUCT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const ART: &[u8] = include_bytes!("../../assets/splash/splash.jpg");
const W: f32 = 480.0;
const H: f32 = 320.0;
const STRIP: f32 = 84.0;
const SHOW_MS: u32 = 1500;
const FADE_MS: u32 = 120;
const FADE_STEPS: u32 = 6;
const TIMER_HOLD: usize = 1;
const TIMER_FADE: usize = 2;
const CLASS: &str = "LikhonSplash";

struct Splash {
    hwnd: HWND,
    gfx: Gfx,
    target: Option<ID2D1HwndRenderTarget>,
    art: Option<ID2D1Bitmap>,
    lang: Lang,
    fade: u32,
}

thread_local! {
    static SPLASH: RefCell<Option<Splash>> = const { RefCell::new(None) };
}

/// Shows the splash on the primary screen. Call on the agent's thread; the agent's message loop drives it.
pub fn show(lang: Lang) {
    let Some(gfx) = Gfx::new() else { return };
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let name = wide(CLASS);
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
        // No focus: WS_EX_NOACTIVATE, shown with SW_SHOWNOACTIVATE, and WM_MOUSEACTIVATE answers MA_NOACTIVATE.
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            name.as_ptr(),
            wide("Likhon").as_ptr(),
            WS_POPUP,
            0,
            0,
            10,
            10,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return;
        }
        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
        let round: u32 = 2; // DWMWCP_ROUND: Windows 11 rounds the corners like the design; Windows 10 ignores it
        DwmSetWindowAttribute(hwnd, 33, (&round as *const u32).cast(), 4);

        // Centre on the primary screen's work area, in that screen's scale.
        let mon = MonitorFromPoint(windows_sys::Win32::Foundation::POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(mon, &mut mi);
        let k = GetDpiForWindow(hwnd) as f32 / 96.0;
        let (w, h) = ((W * k).round() as i32, (H * k).round() as i32);
        let wa: RECT = mi.rcWork;
        let x = wa.left + (wa.right - wa.left - w) / 2;
        let y = wa.top + (wa.bottom - wa.top - h) / 2;
        SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);

        SPLASH.with(|s| *s.borrow_mut() = Some(Splash { hwnd, gfx, target: None, art: None, lang, fade: 0 }));
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        UpdateWindow(hwnd);
        SetTimer(hwnd, TIMER_HOLD, SHOW_MS, None);
    }
}

fn close() {
    let hwnd = SPLASH.with(|s| s.borrow_mut().take().map(|s| s.hwnd));
    if let Some(h) = hwnd {
        unsafe { DestroyWindow(h) };
    }
}

/// Decodes the bundled JPEG with WIC into a bitmap for this render target.
fn load_art(rt: &ID2D1HwndRenderTarget) -> Option<ID2D1Bitmap> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let stream = wic.CreateStream().ok()?;
        stream.InitializeFromMemory(ART).ok()?;
        let decoder = wic.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand).ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let conv = wic.CreateFormatConverter().ok()?;
        conv.Initialize(&frame, &GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeMedianCut).ok()?;
        rt.CreateBitmapFromWicBitmap(&conv, None).ok()
    }
}

fn bn_digits(s: &str) -> String {
    s.chars().map(|c| c.to_digit(10).map(|d| char::from_u32(0x09E6 + d).unwrap()).unwrap_or(c)).collect()
}

fn paint(sp: &mut Splash) {
    unsafe {
        if sp.target.is_none() {
            let mut rc = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(sp.hwnd, &mut rc);
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                // Software, like the top bar: no GPU driver loaded into the agent for a window shown once.
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT { format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN, alphaMode: D2D1_ALPHA_MODE_UNKNOWN },
                dpiX: 0.0,
                dpiY: 0.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let px = D2D_SIZE_U { width: (rc.right - rc.left).max(1) as u32, height: (rc.bottom - rc.top).max(1) as u32 };
            let hprops = D2D1_HWND_RENDER_TARGET_PROPERTIES { hwnd: windows::Win32::Foundation::HWND(sp.hwnd), pixelSize: px, presentOptions: D2D1_PRESENT_OPTIONS_NONE };
            sp.target = sp.gfx.d2d.CreateHwndRenderTarget(&props, &hprops).ok();
            if let Some(t) = &sp.target {
                let dpi = GetDpiForWindow(sp.hwnd) as f32;
                t.SetDpi(dpi, dpi);
                sp.art = load_art(t);
            }
        }
        let Some(rt) = sp.target.clone() else { return };
        rt.BeginDraw();
        // Behind the art, the design's sunrise colours, in case the image cannot be decoded.
        rt.Clear(Some(&rgb(0x67B4F1)));
        let full = D2D_RECT_F { left: 0.0, top: 0.0, right: W, bottom: H };
        if let Some(art) = &sp.art {
            rt.DrawBitmap(art, Some(&full), 1.0, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
        }
        let p = Painter { rt: &rt, gfx: &sp.gfx };
        // The cream strip with the mark, name, tagline and version (light tokens: the splash has no dark version).
        p.round_rect(D2D_RECT_F { left: 0.0, top: 0.0, right: W, bottom: STRIP }, 0.0, Some(tokens::light::BG), None);
        p.round_rect(D2D_RECT_F { left: 20.0, top: 18.0, right: 68.0, bottom: 66.0 }, 11.0, Some(tokens::light::ACCENT), None);
        p.quill(27.0, 25.0, 34.0, tokens::light::ON);
        p.text(&t("app.splash.name", sp.lang, &[]), 82.0, 14.0, 46.0, 22.0, 600, tokens::light::INK);
        p.text(&t("app.splash.tagline", sp.lang, &[]), 82.0, 44.0, 68.0, 14.0, 400, tokens::light::INK);
        let v = env!("CARGO_PKG_VERSION");
        let version = t("app.splash.version", sp.lang, &[("version", &if sp.lang == Lang::Bn { bn_digits(v) } else { v.to_string() })]);
        if let Some((_, w, _)) = text_layout(&sp.gfx.dw, &version, 12.0, 400) {
            p.text(&version, W - 20.0 - w, 18.0, 66.0, 12.0, 400, tokens::light::MUTED);
        }
        if rt.EndDraw(None, None).is_err() {
            sp.target = None;
            sp.art = None;
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
        WM_LBUTTONDOWN => {
            close();
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            BeginPaint(hwnd, &mut ps);
            SPLASH.with(|s| {
                if let Ok(mut s) = s.try_borrow_mut() {
                    if let Some(sp) = s.as_mut() {
                        paint(sp);
                    }
                }
            });
            EndPaint(hwnd, &ps);
            0
        }
        WM_TIMER if wparam == TIMER_HOLD => {
            KillTimer(hwnd, TIMER_HOLD);
            SetTimer(hwnd, TIMER_FADE, FADE_MS / FADE_STEPS, None);
            0
        }
        WM_TIMER if wparam == TIMER_FADE => {
            let step = SPLASH.with(|s| s.borrow_mut().as_mut().map(|sp| {
                sp.fade += 1;
                sp.fade
            }));
            match step {
                Some(n) if n < FADE_STEPS => {
                    SetLayeredWindowAttributes(hwnd, 0, (255 * (FADE_STEPS - n) / FADE_STEPS) as u8, LWA_ALPHA);
                }
                _ => {
                    KillTimer(hwnd, TIMER_FADE);
                    close();
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
