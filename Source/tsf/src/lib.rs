// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//! likhon_tsf.dll: Likhon as a Windows text service (Desktop/CLAUDE.md 5, "Text-service path"; milestone M3).
//!
//! Registered for Bangla (Bangladesh), so it is listed in Settings › Time & language › Bangla › Keyboards and in
//! Win+Space. While it is the active keyboard it types the word being written as a composition (the app shows it in
//! place; Backspace edits it key by key) and ends it on Space, Enter, punctuation the layout does not use, or a click.
//! It always types Bangla (owner decision 2026-10-24): there is no toggle key; English is another keyboard in
//! Win+Space. likhon.exe's own hook stands aside whenever the focused app's keyboard is a Bangla one
//! (app/src/agent.rs), so text is never converted twice.
//!
//! It follows Likhon's settings: the layout (Settings › General, the top bar) and its options come from settings.json,
//! read again when it changed, checked at the start of each word and when the app gains focus. While a word is typed,
//! the keyboard shows its own suggestion window (suggest.rs: dictionary and My words read-only, with or without
//! likhon.exe running); Up and Down pick, Space and Enter take the picked word, Escape closes the list. The taskbar's input indicator and Win+Space show the keyboard's icon, the Likhon logo
//! (icon 1 in likhon-tsf.rc). No language bar button: Windows showed it as a second icon beside that one.
//!
//! It loads into other programs, so it stays thin: no WebView, no network, nothing written (SQLite only reads).
//! A panic here would close the host app (the release profile aborts on panic): no unwrap, no RefCell borrow held
//! across a call into Windows.

pub mod compose;
mod register;
mod suggest;

use compose::{Composer, Key};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::TextServices::*;
use windows_core::{implement, ComObject, IUnknownImpl, Interface, Ref, BOOL, GUID, HRESULT};

/// The text service's class id and its keyboard profile id. Never change them: Windows keeps them in the registry.
pub const CLSID_LIKHON: GUID = GUID::from_u128(0xc772dc88_b930_41b1_a45a_c800a07fad7c);
pub const PROFILE_LIKHON: GUID = GUID::from_u128(0x1a047b60_a777_42b7_b08b_2329083dcfe3);
/// Bangla (Bangladesh).
pub const LANGID_BN_BD: u16 = 0x0845;

static OBJECTS: AtomicUsize = AtomicUsize::new(0);

fn down(vk: VIRTUAL_KEY) -> bool {
    unsafe { GetKeyState(vk.0 as i32) < 0 }
}

// ---- The text service ---------------------------------------------------------------------------------------------

#[implement(ITfTextInputProcessorEx, ITfKeyEventSink, ITfCompositionSink)]
pub struct TextService {
    tm: RefCell<Option<ITfThreadMgr>>,
    tid: Cell<u32>,
    composition: RefCell<Option<ITfComposition>>,
    composer: RefCell<Composer>,
    /// Where the word is on screen after the last edit, for the suggestion window.
    at: Cell<Option<RECT>>,
    /// The focused field says it is a password field (input scope): no suggestions, nothing learned.
    password: Cell<bool>,
    /// settings.json's last change time, to read it again only when it changed.
    stamp: Cell<Option<std::time::SystemTime>>,
}

impl TextService {
    fn new() -> Self {
        OBJECTS.fetch_add(1, Ordering::SeqCst);
        let s = Self {
            tm: RefCell::new(None),
            tid: Cell::new(0),
            composition: RefCell::new(None),
            composer: RefCell::new(Composer::new(compose::Scheme::Phonetic(Default::default()))),
            at: Cell::new(None),
            password: Cell::new(false),
            stamp: Cell::new(None),
        };
        s.reload(true);
        s
    }

    fn dirs() -> Vec<std::path::PathBuf> {
        compose::data_dirs(register::module_path().and_then(|p| p.parent().map(|d| d.to_path_buf())))
    }

    /// Reads settings.json again if it changed (or always, when `force`).
    fn reload(&self, force: bool) {
        let dirs = Self::dirs();
        let stamp = dirs.iter().find_map(|d| std::fs::metadata(d.join("settings.json")).ok()?.modified().ok());
        if !force && stamp == self.stamp.get() {
            return;
        }
        self.stamp.set(stamp);
        let l = compose::load(&dirs);
        if let Ok(mut c) = self.composer.try_borrow_mut() {
            *c = Composer::new(l.scheme);
        }
        let dll_dir = register::module_path().and_then(|p| p.parent().map(|d| d.to_path_buf()));
        suggest::configure(dll_dir.as_deref(), &dirs);
    }

    fn composing(&self) -> bool {
        self.composition.try_borrow().map(|c| c.is_some()).unwrap_or(false)
    }

    fn word(&self) -> String {
        self.composer.try_borrow().map(|c| c.text()).unwrap_or_default()
    }

    /// What a key press means to the service right now.
    fn decide(&self, vk: u16) -> Decision {
        let (ctrl, alt, shift) = (down(VK_CONTROL), down(VK_MENU), down(VK_SHIFT));
        if matches!(VIRTUAL_KEY(vk), VK_SHIFT | VK_LSHIFT | VK_RSHIFT | VK_CONTROL | VK_LCONTROL | VK_RCONTROL | VK_MENU | VK_LMENU | VK_RMENU | VK_CAPITAL) {
            return Decision::Ignore;
        }
        let composing = self.composing();
        if !composing {
            // A new word: pick up a layout or option changed from the top bar or Settings.
            self.reload(false);
        }
        // The suggestion list is open: Up and Down move, Escape closes, Enter takes a picked word that differs.
        if composing && suggest::showing() && !ctrl && !alt {
            match VIRTUAL_KEY(vk) {
                VK_UP => return Decision::Step(false),
                VK_DOWN => return Decision::Step(true),
                VK_ESCAPE => return Decision::Close,
                VK_RETURN if !shift && suggest::selected().is_some_and(|w| w != self.word()) => return Decision::Enter,
                _ => {}
            }
        }
        let altgr = ctrl && alt && down(VK_RMENU);
        if (ctrl || alt) && !altgr {
            return if composing { Decision::EndAndPass } else { Decision::Ignore };
        }
        if composing && vk == VK_BACK.0 {
            return Decision::Back;
        }
        if composing && vk == VK_SPACE.0 && !shift {
            return Decision::Space;
        }
        let caps = unsafe { GetKeyState(VK_CAPITAL.0 as i32) } & 1 == 1;
        let Some(k) = compose::key(vk, shift, caps, altgr) else {
            return if composing { Decision::EndAndPass } else { Decision::Ignore };
        };
        let fits = self.composer.try_borrow().ok().and_then(|c| c.with(k)).is_some();
        match (fits, composing) {
            (true, _) => Decision::Type(k),
            (false, true) => Decision::EndAndPass,
            (false, false) => Decision::Ignore,
        }
    }

    /// Runs an edit on the document: synchronously when Windows allows it (in OnKeyDown), else soon after.
    fn edit(this: &ComObject<TextService>, ctx: &ITfContext, op: Op, sync: bool) {
        let session: ITfEditSession = Session { svc: this.clone(), ctx: ctx.clone(), op }.into();
        let flags = if sync { TF_ES_SYNC | TF_ES_READWRITE } else { TF_ES_ASYNCDONTCARE | TF_ES_READWRITE };
        unsafe {
            let _ = ctx.RequestEditSession(this.tid.get(), &session, flags);
        }
    }

    /// The word on screen changed: ask likhon.exe for suggestions where it is. Apps often have no layout for text
    /// changed in the same edit (TS_E_NOLAYOUT), so the position is measured in a follow-up edit (Op::Measure); without
    /// one, likhon.exe places the list by the caret as for the hook.
    fn suggest(&self) {
        let (keys, text) = self.composer.try_borrow().map(|c| (c.typed(), c.text())).unwrap_or_default();
        if text.is_empty() {
            suggest::hide();
        } else {
            suggest::show(&keys, &text, self.at.get(), self.password.get());
        }
    }

    /// The word is finished with `word` (picked or as typed): offered for learning (never from a password field), the
    /// list closes.
    fn finished(&self, word: &str) {
        if !self.password.get() {
            suggest::commit(word);
        }
        suggest::hide();
    }

    /// A click on a suggestion row (suggest.rs): the word replaces the one being typed, in the focused document.
    fn pick(this: &ComObject<TextService>, word: String) {
        let tm = this.tm.try_borrow().ok().and_then(|t| t.clone());
        let ctx = tm.and_then(|t| unsafe { t.GetFocus().ok() }).and_then(|d| unsafe { d.GetTop().ok() });
        if let Some(ctx) = ctx {
            this.finished(&word);
            TextService::edit(this, &ctx, Op::EndWith(word, String::new()), false);
        }
    }

    /// Inside an edit session: show `text` as the composition (starting one at the caret if needed), optionally end it
    /// and type `after` behind it.
    fn apply(&self, this: &ComObject<TextService>, ec: u32, ctx: &ITfContext, op: &Op) -> windows_core::Result<()> {
        let (text, end, after) = match op {
            Op::Measure => (None, false, None),
            Op::Show(t) => (Some(t.as_str()), false, None),
            Op::End => (None, true, None),
            Op::EndWith(t, a) => (Some(t.as_str()), true, Some(a.as_str())),
        };
        let current = self.composition.try_borrow().ok().and_then(|c| c.clone());
        unsafe {
            let comp = match current {
                Some(c) => Some(c),
                None if text.is_some_and(|t| !t.is_empty()) => {
                    let ins: ITfInsertAtSelection = ctx.cast()?;
                    let range = ins.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
                    let cc: ITfContextComposition = ctx.cast()?;
                    let sink: ITfCompositionSink = this.to_interface();
                    let c = cc.StartComposition(ec, &range, &sink)?;
                    if let Ok(mut slot) = self.composition.try_borrow_mut() {
                        *slot = Some(c.clone());
                    }
                    Some(c)
                }
                None => None,
            };
            if let Some(c) = &comp {
                let range = c.GetRange()?;
                if let Some(t) = text {
                    let w: Vec<u16> = t.encode_utf16().collect();
                    range.SetText(ec, 0, &w)?;
                }
                // Where the word is on screen, for the suggestion window.
                if !end {
                    let mut rc = RECT::default();
                    let mut clipped = BOOL(0);
                    let found = ctx.GetActiveView().and_then(|v| v.GetTextExt(ec, &range, &mut rc, &mut clipped)).is_ok();
                    self.at.set(found.then_some(rc).filter(|r| r.bottom > r.top));
                }
                if matches!(op, Op::Measure) {
                    self.password.set(password_field(ec, ctx, &range));
                    self.suggest();
                    return Ok(());
                }
                // The caret after the word.
                let caret = range.Clone()?;
                caret.Collapse(ec, TF_ANCHOR_END)?;
                let sel = TF_SELECTION { range: core::mem::ManuallyDrop::new(Some(caret)), style: TF_SELECTIONSTYLE { ase: TF_AE_NONE, fInterimChar: false.into() } };
                let _ = ctx.SetSelection(ec, &[sel.clone()]);
                drop(core::mem::ManuallyDrop::into_inner(sel.range));
                if end {
                    if let Ok(mut slot) = self.composition.try_borrow_mut() {
                        *slot = None;
                    }
                    c.EndComposition(ec)?;
                }
            }
            if end {
                if let Ok(mut k) = self.composer.try_borrow_mut() {
                    k.clear();
                }
                self.at.set(None);
            }
            if let Some(a) = after.filter(|a| !a.is_empty()) {
                let ins: ITfInsertAtSelection = ctx.cast()?;
                let w: Vec<u16> = a.encode_utf16().collect();
                let r = ins.InsertTextAtSelection(ec, INSERT_TEXT_AT_SELECTION_FLAGS(0), &w)?;
                r.Collapse(ec, TF_ANCHOR_END)?;
                let sel = TF_SELECTION { range: core::mem::ManuallyDrop::new(Some(r)), style: TF_SELECTIONSTYLE { ase: TF_AE_NONE, fInterimChar: false.into() } };
                let _ = ctx.SetSelection(ec, &[sel.clone()]);
                drop(core::mem::ManuallyDrop::into_inner(sel.range));
            }
        }
        Ok(())
    }
}

impl Drop for TextService {
    fn drop(&mut self) {
        OBJECTS.fetch_sub(1, Ordering::SeqCst);
    }
}

#[derive(Clone, Copy)]
enum Decision {
    Ignore,
    Type(Key),
    Back,
    Space,
    EndAndPass,
    /// Up (false) or Down (true) in the suggestion list.
    Step(bool),
    /// Escape: close the suggestion list.
    Close,
    /// Enter: take the picked suggestion.
    Enter,
}

enum Op {
    /// Read where the word is on screen, then ask for suggestions (after the app laid the new text out).
    Measure,
    Show(String),
    End,
    /// End with this text, then type the second text behind it (a space).
    EndWith(String, String),
}

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<ITfThreadMgr>, tid: u32) -> windows_core::Result<()> {
        self.ActivateEx(ptim, tid, 0)
    }

    fn Deactivate(&self) -> windows_core::Result<()> {
        // The suggestion window's link held the service: let both go.
        suggest::close();
        let tm = self.tm.try_borrow_mut().ok().and_then(|mut t| t.take());
        if let Some(tm) = tm {
            if let Ok(km) = tm.cast::<ITfKeystrokeMgr>() {
                unsafe {
                    let _ = km.UnadviseKeyEventSink(self.tid.get());
                }
            }
        }
        if let Ok(mut c) = self.composition.try_borrow_mut() {
            *c = None;
        }
        if let Ok(mut k) = self.composer.try_borrow_mut() {
            k.clear();
        }
        Ok(())
    }
}

impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    fn ActivateEx(&self, ptim: Ref<ITfThreadMgr>, tid: u32, _flags: u32) -> windows_core::Result<()> {
        let tm = ptim.ok()?.clone();
        self.tid.set(tid);
        let sink: ITfKeyEventSink = self.to_object().to_interface();
        unsafe { tm.cast::<ITfKeystrokeMgr>()?.AdviseKeyEventSink(tid, &sink, true)? };
        if let Ok(mut t) = self.tm.try_borrow_mut() {
            *t = Some(tm);
        }
        // Suggestions: a click on a row in likhon.exe's window comes back here.
        let this = self.to_object();
        suggest::open(Rc::new(move |word| TextService::pick(&this, word)));
        self.reload(true);
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, foreground: BOOL) -> windows_core::Result<()> {
        if foreground.as_bool() {
            self.reload(false);
        } else {
            suggest::hide();
        }
        Ok(())
    }

    fn OnTestKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> windows_core::Result<BOOL> {
        Ok(match self.decide(wparam.0 as u16) {
            Decision::Ignore => false,
            Decision::EndAndPass => {
                // The key goes to the app; the word is finished first (just after, if Windows will not edit now).
                if let Some(ctx) = pic.as_ref() {
                    suggest::hide();
                    TextService::edit(&self.to_object(), ctx, Op::End, false);
                }
                false
            }
            _ => true,
        }
        .into())
    }

    fn OnKeyDown(&self, pic: Ref<ITfContext>, wparam: WPARAM, _lparam: LPARAM) -> windows_core::Result<BOOL> {
        let Some(ctx) = pic.as_ref() else { return Ok(false.into()) };
        let this = self.to_object();
        let vk = wparam.0 as u16;
        let eaten = match self.decide(vk) {
            Decision::Ignore => false,
            Decision::EndAndPass => {
                // Enter and Tab finish the word as typed (it may be learned); other keys just end it.
                if matches!(VIRTUAL_KEY(vk), VK_RETURN | VK_TAB) {
                    self.finished(&self.word());
                } else {
                    suggest::hide();
                }
                TextService::edit(&this, ctx, Op::End, true);
                false
            }
            Decision::Type(k) => {
                let text = self.composer.try_borrow_mut().map(|mut c| c.push(k)).unwrap_or_default();
                TextService::edit(&this, ctx, Op::Show(text), true);
                TextService::edit(&this, ctx, Op::Measure, false);
                true
            }
            Decision::Back => {
                let (text, empty) = self.composer.try_borrow_mut().map(|mut c| (c.pop(), c.is_empty())).unwrap_or_default();
                if empty {
                    suggest::hide();
                    TextService::edit(&this, ctx, Op::EndWith(String::new(), String::new()), true);
                } else {
                    TextService::edit(&this, ctx, Op::Show(text), true);
                    TextService::edit(&this, ctx, Op::Measure, false);
                }
                true
            }
            Decision::Space => {
                // The picked suggestion, else the word as typed.
                let word = suggest::selected().unwrap_or_else(|| self.word());
                self.finished(&word);
                TextService::edit(&this, ctx, Op::EndWith(word, " ".into()), true);
                true
            }
            Decision::Enter => {
                let word = suggest::selected().unwrap_or_else(|| self.word());
                self.finished(&word);
                TextService::edit(&this, ctx, Op::EndWith(word, String::new()), true);
                true
            }
            Decision::Step(down) => {
                suggest::step(down);
                true
            }
            Decision::Close => {
                suggest::hide();
                true
            }
        };
        Ok(eaten.into())
    }

    fn OnTestKeyUp(&self, _pic: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> windows_core::Result<BOOL> {
        Ok(false.into())
    }

    fn OnKeyUp(&self, _pic: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> windows_core::Result<BOOL> {
        Ok(false.into())
    }

    fn OnPreservedKey(&self, _pic: Ref<ITfContext>, _rguid: *const GUID) -> windows_core::Result<BOOL> {
        Ok(false.into())
    }
}

impl ITfCompositionSink_Impl for TextService_Impl {
    /// The app or Windows ended the word (a click elsewhere, focus moved): start fresh next time.
    fn OnCompositionTerminated(&self, _ec: u32, _comp: Ref<ITfComposition>) -> windows_core::Result<()> {
        if let Ok(mut c) = self.composition.try_borrow_mut() {
            *c = None;
        }
        if let Ok(mut k) = self.composer.try_borrow_mut() {
            k.clear();
        }
        self.at.set(None);
        suggest::hide();
        Ok(())
    }
}

/// Whether the app marks the field as a password field (TSF input scope IS_PASSWORD). Inside an edit session.
fn password_field(ec: u32, ctx: &ITfContext, range: &ITfRange) -> bool {
    const IS_PASSWORD: i32 = 31;
    unsafe {
        let Ok(prop) = ctx.GetAppProperty(&GUID_PROP_INPUTSCOPE) else { return false };
        let Ok(v) = prop.GetValue(ec, range) else { return false };
        let Ok(unk) = windows_core::IUnknown::try_from(&v) else { return false };
        let Ok(scope) = unk.cast::<ITfInputScope>() else { return false };
        let mut list: *mut InputScope = core::ptr::null_mut();
        let mut n = 0u32;
        if scope.GetInputScopes(&mut list, &mut n).is_err() || list.is_null() {
            return false;
        }
        let found = std::slice::from_raw_parts(list, n as usize).iter().any(|s| s.0 == IS_PASSWORD);
        CoTaskMemFree(Some(list as *const _));
        found
    }
}

#[implement(ITfEditSession)]
struct Session {
    svc: ComObject<TextService>,
    ctx: ITfContext,
    op: Op,
}

impl ITfEditSession_Impl for Session_Impl {
    fn DoEditSession(&self, ec: u32) -> windows_core::Result<()> {
        self.svc.apply(&self.svc, ec, &self.ctx, &self.op)
    }
}

// ---- The class factory and the DLL's exports ---------------------------------------------------------------------

#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(&self, outer: Ref<windows_core::IUnknown>, riid: *const GUID, ppv: *mut *mut core::ffi::c_void) -> windows_core::Result<()> {
        if outer.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        let unknown: windows_core::IUnknown = TextService::new().into();
        unsafe { unknown.query(riid, ppv).ok() }
    }

    fn LockServer(&self, lock: BOOL) -> windows_core::Result<()> {
        if lock.as_bool() {
            OBJECTS.fetch_add(1, Ordering::SeqCst);
        } else {
            OBJECTS.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(rclsid: *const GUID, riid: *const GUID, ppv: *mut *mut core::ffi::c_void) -> HRESULT {
    if ppv.is_null() || rclsid.is_null() {
        return E_POINTER;
    }
    *ppv = core::ptr::null_mut();
    if *rclsid != CLSID_LIKHON {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = Factory.into();
    factory.query(riid, ppv)
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if OBJECTS.load(Ordering::SeqCst) == 0 { S_OK } else { S_FALSE }
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    match register::register() {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    match register::unregister() {
        Ok(()) => S_OK,
        Err(e) => e.code(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_factory_makes_a_text_service_with_its_interfaces() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let mut f: *mut core::ffi::c_void = core::ptr::null_mut();
            assert_eq!(DllGetClassObject(&CLSID_LIKHON, &IClassFactory::IID, &mut f), S_OK);
            let factory = IClassFactory::from_raw(f);
            let tip: ITfTextInputProcessorEx = factory.CreateInstance(None).expect("instance");
            assert!(tip.cast::<ITfTextInputProcessor>().is_ok());
            assert!(tip.cast::<ITfKeyEventSink>().is_ok());
            assert!(tip.cast::<ITfCompositionSink>().is_ok());
            let wrong = GUID::from_u128(1);
            assert_eq!(DllGetClassObject(&wrong, &IClassFactory::IID, &mut f), CLASS_E_CLASSNOTAVAILABLE);
            drop(tip);
            drop(factory);
            assert_eq!(DllCanUnloadNow(), S_OK);
        }
    }
}
