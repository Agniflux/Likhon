// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Likhon Settings window (likhon.exe --settings [page]): Reference/design-v2/Likhon Desktop.dc.html §7.
//
// Plain TypeScript, no framework. The Rust side (app/src/settings_app.rs) serves these files from inside
// the exe, answers `call()` requests over the web view's IPC channel, writes settings.json, and tells the
// running agent to reload it, so changes apply at once. The window has no Windows frame: the title bar,
// minimize, close and the resize edges here ask the exe to move the window. The exe goes online only when a button
// here asks it to (Updates, Library, Donate words), and only to the Likhon site; links open in the default browser.

type Lang = 'en' | 'bn';
type Json = null | boolean | number | string | Json[] | { [k: string]: Json };
type Child = Node | string | null | undefined | false;
type Hotkey = { vk: number; ctrl: boolean; alt: boolean; shift: boolean; label: string };
type App = { name: string; exe: string };

interface Boot {
  settings: Record<string, Json>;
  defaults: Record<string, Json>;
  page: string;
  version: string;
  edition: 'portable' | 'native';
  /** The date of the current software license agreement (first run shows it; About names it). */
  agreementDate: string;
  /** The Likhon site (likhon.agniflux.com by default). */
  site: string;
  /** Whether this Windows user's Run entry starts this likhon.exe. */
  startWithWindows: boolean;
  /** How many words Likhon has learned. */
  learned: number;
  /** Likhon is registered as a Windows keyboard (likhon_tsf.dll). */
  windowsKeyboard: boolean;
  windowsKeyboardStale?: boolean;
  windows: string;
  dataDir: string;
  dataPortable: boolean;
  diagnostics: string | null;
  /** The dictionary in the dict folder beside likhon.exe, or null when it is missing. */
  dictionary: { words: number; path: string; license: string; snapshot?: string | null } | null;
}

declare global {
  interface Window {
    ipc: { postMessage(message: string): void };
    __likhon: { reply(id: number, data: Json): void; maximized(on: boolean): void };
  }
}

// ---- Bridge to the exe ----------------------------------------------------------------------------

const pending = new Map<number, (v: Json) => void>();
let seq = 0;
function call<T extends Json = Json>(cmd: string, args: Record<string, Json> = {}): Promise<T> {
  const id = ++seq;
  return new Promise((resolve) => {
    pending.set(id, resolve as (v: Json) => void);
    window.ipc.postMessage(JSON.stringify({ id, cmd, args }));
  });
}
window.__likhon = {
  reply(id, data) {
    pending.get(id)?.(data);
    pending.delete(id);
  },
  // The exe reports maximize and restore (the button, a title bar double-click, Win+Up, snapping).
  maximized(on) {
    maximized = on;
    document.body.classList.toggle('max', on);
    renderTitlebar();
  },
};

// ---- Data -----------------------------------------------------------------------------------------

let boot: Boot;
let text: Record<Lang, Record<string, string>>;
let icons: Record<string, [string, Record<string, string>][]>;
let S: Record<string, Json> = {};
let page = 'general';
let query = '';
let maximized = false;

const get = <T extends Json>(key: string): T => (key in S ? S[key] : boot.defaults[key]) as T;
const lang = (): Lang => (get<string>('language') === 'bn' ? 'bn' : 'en');

async function set(key: string, value: Json, rerender = true): Promise<void> {
  S[key] = value;
  await call('set', { key, value });
  if (rerender) render();
}

async function reset(keys: string[]): Promise<void> {
  for (const k of keys) delete S[k];
  await call('reset', { keys });
  applyTheme();
  render();
}

/** Opens a page of the Likhon site in the default browser (the exe refuses any other address). */
const site = (path: string) => call('openUrl', { url: `${boot.site}/${lang()}/${path}` });
const host = () => boot.site.replace(/^https:\/\//, '');

// ---- Text -----------------------------------------------------------------------------------------

function t(id: string, vars: Record<string, string | number> = {}): string {
  const full = id.startsWith('app.') ? id : 'app.set.' + id;
  let s = text[lang()][full] ?? text.en[full] ?? full;
  for (const [k, v] of Object.entries(vars)) s = s.split('{' + k + '}').join(String(v));
  return s;
}

const BN_DIGITS = '০১২৩৪৫৬৭৮৯';
const bnDigits = (s: string) => s.replace(/[0-9]/g, (d) => BN_DIGITS[+d]);
const num = (n: number | string) => (lang() === 'bn' ? bnDigits(String(n)) : String(n));

// ---- DOM helpers ----------------------------------------------------------------------------------

type Props = Record<string, unknown>;
function h<K extends keyof HTMLElementTagNameMap>(tag: K, props: Props = {}, ...kids: (Child | Child[])[]): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (v === undefined || v === null || v === false) continue;
    if (k === 'class') el.className = String(v);
    else if (k.startsWith('on') && typeof v === 'function') el.addEventListener(k.slice(2), v as EventListener);
    else if (k === 'value' || k === 'checked' || k === 'disabled') (el as unknown as Record<string, unknown>)[k] = v;
    else el.setAttribute(k, v === true ? '' : String(v));
  }
  for (const kid of kids.flat()) if (kid !== null && kid !== undefined && kid !== false) el.append(kid);
  return el;
}

const SVG = 'http://www.w3.org/2000/svg';
function icon(name: string, size = 16, cls = ''): SVGSVGElement {
  const svg = document.createElementNS(SVG, 'svg');
  for (const [k, v] of Object.entries({ width: size, height: size, viewBox: name === 'quill' ? '150 150 900 900' : '0 0 24 24', fill: 'none', 'aria-hidden': 'true' })) {
    svg.setAttribute(k, String(v));
  }
  svg.setAttribute('class', ('i ' + (name === 'quill' ? 'quill ' : '') + cls).trim());
  for (const [tag, attrs] of icons[name] ?? []) {
    const el = document.createElementNS(SVG, tag);
    for (const [k, v] of Object.entries(attrs)) if (k !== 'key') el.setAttribute(k, v);
    svg.append(el);
  }
  return svg;
}

// ---- Rows (the v2 Settings vocabulary) ------------------------------------------------------------

const soon = () => h('span', { class: 'soon', title: t('soonTip') }, t('soon'));

/** One settings row: label and description on the left, the control on the right. */
function row(label: string, desc: Child | Child[], control: Child | Child[], later = false) {
  return h('div', { class: 'r' + (later ? ' later' : ''), 'data-find': label },
    h('div', { class: 'lbl' }, h('div', { class: 'l' }, label, later ? soon() : null), desc ? h('div', { class: 'd' }, desc) : null),
    h('div', { class: 'ctl' }, control));
}
/** A row with its controls under the text (for controls too wide to sit beside it), plus an optional message under both. */
function stacked(r: HTMLElement, msg?: Node | null) {
  r.classList.add('stack');
  if (msg) r.append(msg);
  return r;
}
/** A full-width part of the page (a note, a table, cards), with an optional label for the finder. */
const block = (node: Node, find?: string) => h('div', { class: 'r wide', 'data-find': find }, node);

function toggle(key: string, label: string, locked = false) {
  return toggleView(!!get<boolean>(key), label, locked, () => set(key, !get<boolean>(key)));
}
function toggleView(on: boolean, label: string, locked: boolean, onclick?: () => void) {
  return h('button', { type: 'button', class: 'switch', role: 'switch', 'aria-checked': String(on), 'aria-label': label, disabled: locked || !onclick, onclick });
}

function seg<T extends string>(aria: string, current: T | null, items: { v: T; label: string }[], onPick?: (v: T) => void) {
  return h('div', { class: 'seg', role: 'radiogroup', 'aria-label': aria },
    items.map((it) => h('button', {
      type: 'button', role: 'radio', 'aria-checked': String(it.v === current), disabled: !onPick,
      lang: it.v === 'en' || it.v === 'bn' ? it.v : undefined, onclick: () => onPick?.(it.v),
    }, it.label)));
}

function select<T extends string>(aria: string, current: T, items: { v: T; label: string }[], onPick?: (v: T) => void) {
  return h('span', { class: 'sel' },
    h('select', { 'aria-label': aria, disabled: !onPick, onchange: (e: Event) => onPick?.((e.target as HTMLSelectElement).value as T) },
      items.map((it) => h('option', { value: it.v, selected: it.v === current ? true : undefined }, it.label))),
    icon('chevron-down', 14));
}

function button(label: string, opts: { icon?: string; kind?: 'primary' | 'danger'; onclick?: () => void; title?: string } = {}) {
  return h('button', { type: 'button', class: 'btn' + (opts.kind ? ' ' + opts.kind : ''), disabled: !opts.onclick, title: opts.title, onclick: opts.onclick },
    opts.icon ? icon(opts.icon, 14) : null, label);
}

function slider(key: string, aria: string, min: number, max: number) {
  const n = get<number>(key);
  const out = h('span', { class: 'v' }, num(n));
  return h('span', { class: 'slider' },
    h('input', {
      type: 'range', min, max, value: n, 'aria-label': aria,
      oninput: (e: Event) => { out.textContent = num((e.target as HTMLInputElement).value); },
      onchange: (e: Event) => set(key, +(e.target as HTMLInputElement).value),
    }), out);
}

function note(body: string, tone: 'info' | 'warn' | 'lock' = 'info') {
  return h('div', { class: 'note ' + tone }, icon(tone === 'warn' ? 'alert-triangle' : tone === 'lock' ? 'lock' : 'info', 16), h('span', {}, body));
}

const keys = (label: string) => label.split(' + ').map((k) => h('kbd', {}, k));

/** v2 link cards (Tools, Updates, About). */
type Card = { icon: string; label: string; desc: string; onclick?: () => void; later?: boolean; title?: string };
const cards = (items: Card[], find?: string) => block(h('div', { class: 'cards' }, items.map((c) =>
  h('button', { type: 'button', class: 'card', disabled: !c.onclick, title: c.title ?? (c.later ? t('soonTip') : undefined), onclick: c.onclick },
    icon(c.icon, 20), h('span', { class: 'ct' }, h('b', {}, c.label, c.later ? soon() : null), h('span', {}, c.desc))))), find);

function dialog(title: string, body: string | Node, actions: Child[] = []): () => void {
  const close = () => { scrim.remove(); document.removeEventListener('keydown', esc); };
  const esc = (e: KeyboardEvent) => { if (e.key === 'Escape') close(); };
  const scrim = h('div', { class: 'scrim', onclick: (e: Event) => { if (e.target === scrim) close(); } },
    h('div', { class: 'dialog', role: 'dialog', 'aria-label': title },
      h('div', { class: 'dh' }, title, h('button', { class: 'icon-btn', type: 'button', 'aria-label': t('close'), onclick: close }, icon('x', 14))),
      typeof body === 'string' ? h('pre', {}, body) : body,
      h('div', { class: 'df' }, ...actions, h('button', { class: 'btn', type: 'button', onclick: close }, t('close')))));
  document.addEventListener('keydown', esc);
  document.body.append(scrim);
  return close;
}

// ---- Pages ----------------------------------------------------------------------------------------

type Page = { keys: string[]; rows: Child[] };
// v2 §7's pages, plus the archive pages v2 has no page for (design-index: S05 Hotkeys, S16 Fonts, S14 Learn).
const NAV: [string, string][] = [['general', 'settings'], ['hotkeys', 'command'], ['typing', 'phonetic'], ['layouts', 'keyboard'], ['suggestions', 'list-checks'],
  ['autocorrect', 'spell-check'], ['shortcuts', 'wand-sparkles'], ['appearance', 'palette'], ['fonts', 'type'], ['legacy', 'legacy-mark'], ['library', 'library'],
  ['tools', 'wrench'], ['privacy', 'shield'], ['updates', 'refresh-cw'], ['learn', 'graduation-cap'], ['about', 'info']];
const SUB: Record<string, string> = {
  general: 'general.sub', typing: 'typing.sub', layouts: 'layouts.sub', suggestions: 'sug.sub', autocorrect: 'auto.sub', shortcuts: 'short.sub',
  appearance: 'look.sub', legacy: 'legacy.sub', hotkeys: 'hk.sub', fonts: 'fonts.sub', learn: 'learn.sub', library: 'lib.sub', tools: 'tools.sub', privacy: 'priv.sub', updates: 'upd.sub', about: '',
};
/** Older page names (tray, top bar, links) that now live on a v2 page. */
const MOVED: Record<string, string> = { output: 'legacy', apprules: 'general', words: 'typing', community: 'privacy', diagnostics: 'privacy' };

const PAGES: Record<string, () => Page> = {
  general() {
    return {
      keys: ['start_with_windows', 'hotkeys.toggle', 'hotkeys.next_layout', 'topbar.mode', 'topbar.fade', 'language', 'apprules.passthrough'],
      rows: [
        // Likhon as a Windows keyboard (Win+Space): the text service, added from any copy (owner decision 2026-10-24).
        // Windows asks for administrator permission once (app/src/winkb.rs).
        row(t('general.winKbH'), [boot.windowsKeyboard ? t(boot.windowsKeyboardStale ? 'general.winKbStale' : 'general.winKbOn') : t('general.winKbOff'), winKbMsg ? h('div', { class: 'msg' + (winKbOk ? ' ok' : '') }, winKbMsg) : null],
          [h('span', { class: 'chip' + (boot.windowsKeyboard ? '' : ' warn') }, boot.windowsKeyboard ? t('general.winKbChipOn') : t('general.winKbChipOff')),
            winKbBusy ? button(t('general.winKbBusy'))
              : boot.windowsKeyboard && boot.windowsKeyboardStale ? button(t('general.winKbUpdate'), { kind: 'primary', icon: 'shield', onclick: () => changeWinKb(true) })
              : boot.windowsKeyboard ? button(t('lib.remove'), { kind: 'danger', onclick: () => changeWinKb(false) })
                : button(t('add'), { kind: 'primary', icon: 'shield', onclick: () => changeWinKb(true) }),
            boot.windowsKeyboard ? button(t('general.winKbBtn'), { icon: 'keyboard', onclick: () => call('openWindowsKeyboards') }) : null]),
        // The layout (owner decision 2026-10-24): the same setting as the top bar, the on-screen keyboard and Layouts,
        // and the Likhon keyboard in Win+Space reads it too, so they all stay the same.
        row(t('general.layoutH'), t('general.layoutP'), select(t('general.layoutH'), get<string>('layout'),
          [...['phonetic', 'probhat', 'jatiya'].map((id) => ({ v: id, label: t('layouts.' + id) })),
            ...sideloaded('layouts').map((l) => ({ v: l.id, label: (lang() === 'bn' && l.name.bn) || l.name.en }))], (v) => set('layout', v))),
        // Start with Windows: a Run entry for this Windows user and this likhon.exe (owner decision 2026-10-22: every copy).
        row(t('general.startH'), t('general.startP'), toggleView(boot.startWithWindows, t('general.startH'), false, async () => {
          if (await call('startWithWindows', { on: !boot.startWithWindows })) boot.startWithWindows = !boot.startWithWindows;
          render();
        })),
        toggleRow(t('general.toggleH'), t('general.toggleP')),
        keyRow('hotkeys.next_layout', t('general.nextH'), t('hk.nextLayoutP'), true),
        row(t('general.tbH'), t('general.tbP'), select(t('general.tbH'), get<string>('topbar.mode'),
          (['always', 'bangla', 'compact'] as const).map((v) => ({ v, label: t('general.tb.' + v) })), (v) => set('topbar.mode', v))),
        row(t('general.fadeH'), t('general.fadeP'), seg(t('general.fadeH'), get<string>('topbar.fade'), [{ v: 'never', label: t('general.fade.never') }, { v: '40', label: num(40) + '%' }, { v: 'hide', label: t('general.fade.hide') }], (v) => set('topbar.fade', v))),
        row(t('general.langH'), null, seg(t('general.langH'), lang(), [{ v: 'en', label: 'English' }, { v: 'bn', label: 'বাংলা' }],
          (v) => set('language', v).then(() => call('title', { text: t('window') })))),
        row(t('general.habitsH'), t('general.habitsP'), presetSelect(t('general.habitsH'))),
        passList(),
        row(t('general.dataH'), h('span', {}, h('span', { class: 'mono' }, boot.dataDir), h('br'), t(boot.dataPortable ? 'general.dataPortable' : 'general.dataProfile')),
          button(t('general.openFolder'), { icon: 'folder-open', onclick: () => call('openFolder') })),
      ],
    };
  },

  typing() {
    return {
      keys: ['typing.preselect', 'typing.autocorrect', 'typing.next_word', 'typing.learn', 'typing.emoji', 'typing.bangla_digits', 'typing.dari', 'typing.visarga', 'typing.number_pick'],
      rows: [
        row(t('typing.preH'), t('typing.preP'), seg(t('typing.preH'), get<string>('typing.preselect'), [{ v: 'dict', label: t('typing.pre.dict') }, { v: 'plain', label: t('typing.pre.plain') }], (v) => set('typing.preselect', v))),
        row(t('typing.autoH'), null, toggle('typing.autocorrect', t('typing.autoH'))),
        row(t('typing.nextH'), null, toggle('typing.next_word', t('typing.nextH'))),
        row(t('typing.learnH'), t('typing.learnP'), toggle('typing.learn', t('typing.learnH'))),
        row(t('typing.emojiH'), t('typing.emojiP'), toggle('typing.emoji', t('typing.emojiH'))),
        row(t('typing.digitsH'), null, seg(t('typing.digitsH'), get<boolean>('typing.bangla_digits') ? 'bn' : 'en',
          [{ v: 'bn', label: '১২৩' }, { v: 'en', label: '123' }], (v) => set('typing.bangla_digits', v === 'bn'))),
        row(t('typing.dariH'), t('typing.dariP'), seg(t('typing.dariH'), get<boolean>('typing.dari') ? 'dari' : 'dot',
          [{ v: 'dari', label: '।' }, { v: 'dot', label: '.' }], (v) => set('typing.dari', v === 'dari'))),
        row(t('typing.visargaH'), t('typing.visargaP'), seg(t('typing.visargaH'), get<boolean>('typing.visarga') ? 'visarga' : 'colon',
          [{ v: 'visarga', label: 'ঃ' }, { v: 'colon', label: ':' }], (v) => set('typing.visarga', v === 'visarga'))),
        row(t('typing.numpickH'), t('typing.numpickP'), toggle('typing.number_pick', t('typing.numpickH'))),
        h('div', { class: 'r wide', 'data-find': t('typing.wordsH') },
          h('div', { class: 'lbl' }, h('div', { class: 'l' }, t('typing.wordsH')), h('div', { class: 'd' }, t('typing.wordsP'))),
          userTable('words', t('typing.wordsC1'), t('typing.wordsC2'), t('typing.wordsEmpty'), 'ইচ্ছেমতো', '')),
      ],
    };
  },

  layouts() {
    const current = get<string>('layout');
    // Phonetic is the engine's own; the fixed layouts are JSON files built into the exe (app/src/layouts.rs).
    const all: [string, string, boolean][] = [['phonetic', 'layouts.phoneticP', true], ['probhat', 'layouts.fixedP', true], ['jatiya', 'layouts.jatiyaP', true],
      ];
    return {
      keys: ['layout'],
      rows: [
        block(h('div', { class: 'radios', role: 'radiogroup', 'aria-label': t('layouts.listAria') }, all.map(([id, desc, ready]) =>
          h('label', { class: (id === current ? 'on' : '') + (ready ? '' : ' off'), 'data-find': t('layouts.' + id) },
            h('input', { type: 'radio', name: 'lay', checked: id === current, disabled: !ready, onchange: () => set('layout', id) }),
            h('span', { class: 'rt' }, h('span', {}, t('layouts.' + id)), h('span', { class: 'd' }, t(desc))),
            h('span', { class: 'tag' + (ready ? '' : ' muted') }, id === current ? t('layouts.inUse') : ready ? '' : t('layouts.noFile')))),
          sideloaded('layouts').map((l) => h('label', { class: l.id === current ? 'on' : '', 'data-find': l.name.en },
            h('input', { type: 'radio', name: 'lay', checked: l.id === current, onchange: () => set('layout', l.id) }),
            h('span', { class: 'rt' }, h('span', {}, (lang() === 'bn' && l.name.bn) || l.name.en), h('span', { class: 'd' }, t('side.fromFile'))),
            h('span', { class: 'tag muted' }, l.id === current ? t('layouts.inUse') : '',
              h('button', { type: 'button', class: 'link-btn', onclick: (e: Event) => { e.preventDefault(); removeSideloaded('layouts', l.id, 'layout'); } }, t('side.remove'))))))),
        row(t('layouts.viewerH'), null, button(t('layouts.viewerBtn'), { icon: 'layout', onclick: () => { tool = 'viewer'; go('tools'); } })),
        row(t('layouts.fileH'), sideMsg.layouts || t('layouts.fileP'), button(t('layouts.fileBtn'), { icon: 'file-up', onclick: () => addFromFile('layouts') })),
      ],
    };
  },

  suggestions() {
    return {
      keys: ['typing.suggestions', 'typing.suggestion_count', 'typing.next_count', 'suggestions.position', 'suggestions.caret_badge'],
      rows: [
        row(t('sug.showH'), null, toggle('typing.suggestions', t('sug.showH'))),
        row(t('sug.countH'), null, slider('typing.suggestion_count', t('sug.countH'), 1, 9)),
        row(t('sug.nextCountH'), null, slider('typing.next_count', t('sug.nextCountH'), 1, 9)),
        row(t('sug.posH'), t('sug.posP'), seg(t('sug.posH'), get<string>('suggestions.position'), [{ v: 'follow', label: t('sug.pos.follow') }, { v: 'anchored', label: t('sug.pos.anchored') }], (v) => set('suggestions.position', v))),
        row(t('sug.pwH'), t('sug.pwP'), toggleView(true, t('sug.pwH'), true)),
        row(t('sug.badgeH'), t('sug.badgeP'), toggle('suggestions.caret_badge', t('sug.badgeH'))),
        // Word choices (user.sqlite): imported, never learned from typing.
        row(t('sug.choicesH'), t('sug.choicesP'), null),
        block(userTable('choices', t('sug.choicesC1'), t('sug.choicesC2'), t('sug.choicesEmpty'), 'ami', 'আমি'), t('sug.choicesH')),
      ],
    };
  },

  // Personal autocorrect entries and shortcuts live in user.sqlite in the data folder (Desktop/CLAUDE.md 7).
  autocorrect: () => ({
    keys: ['typing.autocorrect'],
    rows: [
      row(t('typing.autoH'), null, toggle('typing.autocorrect', t('typing.autoH'))),
      block(userTable('autocorrect', t('auto.c1'), t('auto.c2'), t('auto.empty'), 'বিগ্যান', 'বিজ্ঞান'), t('auto.c1')),
      block(note(t('auto.note'), 'warn')),
    ],
  }),
  shortcuts: () => ({
    keys: ['shortcuts.space'],
    rows: [
      block(userTable('shortcuts', t('short.c1'), t('short.c2'), t('short.empty'), ';ty', 'ধন্যবাদ, ভালো থাকবেন।'), t('short.c1')),
      row(t('short.spaceH'), t('short.spaceP'), toggle('shortcuts.space', t('short.spaceH'))),
    ],
  }),

  appearance() {
    const cur = activeTheme();
    // The suggestion font: Windows' own, or a Bangla font installed on this PC (the list the Fonts page shows).
    if (fonts === null) call<string[]>('bnFonts').then((list) => { fonts = list ?? []; if (page === 'appearance') render(); });
    const font = get<string | null>('appearance.font') ?? '';
    const faces = [...new Set([...(fonts ?? []), ...(font ? [font] : [])])].sort();
    // The native windows draw in Hind Siliguri by default (embedded in likhon.exe); this page has the same font.
    const fontFamily = font ? `"${font.replace(/"/g, '')}", "Hind Siliguri"` : '"Hind Siliguri"';
    const paint = (el: HTMLElement, css: Partial<CSSStyleDeclaration>) => { Object.assign(el.style, css); return el; };
    const card = (th: Theme) => h('button', { type: 'button', class: 'theme', role: 'radio', 'aria-checked': String(th.id === cur.id), onclick: () => set('appearance.theme', th.id) },
      paint(h('span', { class: 'desk' }, paint(h('span', { class: 'sug' },
        paint(h('span', {}, 'বিজ্ঞান'), { background: colors(th).sel, color: colors(th).ink }), paint(h('span', {}, 'বিজ্ঞানী'), { color: colors(th).ink })),
      { background: colors(th).surface, borderColor: colors(th).line, fontFamily })), { background: colors(th).desk }),
      h('span', { class: 'nm' }, themeName(th)));
    return {
      keys: ['appearance.theme', 'color', 'appearance.font', 'appearance.font_size', 'appearance.row_height'],
      rows: [
        block(h('div', { class: 'themes', role: 'radiogroup', 'aria-label': t('look.themesAria') }, [...THEMES, ...sideThemes()].map(card)), t('look.themesAria')),
        // Stacked: one Remove button per theme is too wide to sit beside the text.
        sideThemes().length ? h('div', { class: 'r stack' }, h('div', { class: 'lbl' }, h('div', { class: 'l' }, t('side.themesH')), h('div', { class: 'd' }, t('side.fromFile'))),
          h('div', { class: 'ctl' }, sideThemes().map((th) => button(t('side.removeName', { name: th.name.en }), { icon: 'trash-2', onclick: () => removeSideloaded('themes', th.id, 'appearance.theme') })))) : null,
        row(t('look.darkH'), t('look.darkP'), seg(t('look.darkH'), get<string>('color'), [
          { v: 'system', label: t('general.color.system') }, { v: 'light', label: t('general.color.light') }, { v: 'dark', label: t('general.color.dark') }],
        (v) => set('color', v).then(applyTheme))),
        // Likhon's font: every Likhon window, not only the suggestion window (owner decision 2026-10-24).
        row(t('look.fontH'), t('look.fontP'), select(t('look.fontH'), font, [{ v: '', label: t('look.fontDefault') }, ...faces.filter((f) => f !== 'Hind Siliguri').map((f) => ({ v: f, label: f }))],
          (v) => set('appearance.font', v || null))),
        row(t('look.sizeH'), null, slider('appearance.font_size', t('look.sizeH'), 13, 24)),
        row(t('look.rowH'), null, slider('appearance.row_height', t('look.rowH'), 24, 44)),
        row(t('look.fileH'), sideMsg.themes || t('look.fileP'), button(t('layouts.fileBtn'), { icon: 'file-up', onclick: () => addFromFile('themes') })),
        row(t('look.moreH'), t('look.moreP'), button(t('look.moreBtn'), { icon: 'library', onclick: () => site('library'), title: t('opensSite') })),
      ],
    };
  },

  legacy() {
    // Legacy output (owner decision 2026-10-22: Bijoy output is allowed). The agent converts each word as it is typed
    // (app/src/legacy.rs); the encoding is built into the engine (src/mpl, from Avro Keyboard, MPL-2.0).
    // Switching it on starts in Bijoy; then Shift+F12 or the top bar's pill flips Bijoy and Unicode.
    const on = !!get<boolean>('legacy.enabled') || get<string>('output') === 'bijoy';
    return {
      keys: ['legacy.enabled', 'output', 'legacy.encoding'],
      rows: [
        row(t('legacy.onH'), t('legacy.onP'), toggleView(on, t('legacy.onH'), false, async () => {
          await set('output', on ? 'unicode' : 'bijoy', false);
          await set('legacy.enabled', !on);
        })),
        row(t('legacy.encH'), null, select(t('legacy.encH'), get<string>('legacy.encoding') === 'proshika' ? 'proshika' : 'bijoy',
          [{ v: 'bijoy', label: t('legacy.encBijoy') }, { v: 'proshika', label: t('legacy.encProshika') }], (v) => set('legacy.encoding', v))),
        // Proshika's map was read from its fonts without real documents to check against (likhon-engine src/proshika.tsv).
        get<string>('legacy.encoding') === 'proshika' ? block(note(t('legacy.proshikaNote'), 'warn')) : null,
        block(note(t('legacy.warn'), 'warn')),
        row(t('legacy.convH'), t('legacy.convP'), button(t('legacy.convBtn'), { icon: 'unicode-convert', onclick: () => { tool = 'conv'; go('tools'); } })),
      ],
    };
  },

  library: () => libraryPage(),

  tools: () => toolsPage(),
  hotkeys: () => hotkeysPage(),
  fonts: () => fontsPage(),
  learn: () => learnPage(),

  privacy() {
    const made = boot.diagnostics;
    return {
      keys: [],
      rows: [
        block(note(t('priv.note'), 'lock')),
        // Donated words go up for a public vote on the site before reviewers see them (owner decision 2026-10-24).
        row(t('priv.donateH'), t('priv.donateP'), [
          button(t('priv.voteBtn'), { onclick: () => site('dictionary/vote'), title: t('opensSite') }),
          button(t('priv.donateBtn'), { icon: 'hand-heart', onclick: donate })]),
        row(t('priv.diagH'), made ? t('priv.diagSaved', { file: made }) : t('priv.diagP'), [
          button(t('priv.diagPreview'), { onclick: async () => dialog(t('priv.diagPreviewH'), String(await call('diagPreview'))) }),
          made ? button(t('general.openFolder'), { icon: 'folder-open', onclick: () => call('openFolder') }) : null,
          button(t('priv.diagSave'), { icon: 'file-down', onclick: async () => { boot.diagnostics = String(await call('diagMake')); render(); } })]),
        block(h('div', { class: 'two' },
          h('div', { class: 'list yes' }, h('b', {}, icon('circle-check', 16), t('priv.inH')), h('ul', {}, [0, 1, 2, 3, 4].map((i) => h('li', {}, t('priv.in.' + i))))),
          h('div', { class: 'list no' }, h('b', {}, icon('x-circle', 16), t('priv.outH')), h('ul', {}, [0, 1, 2, 3].map((i) => h('li', {}, t('priv.out.' + i))))))),
        row(t('priv.forgetH'), boot.learned ? t('priv.learnedN', { n: num(boot.learned) }) : t('typing.learnedP'),
          button(t('priv.forgetBtn'), { kind: 'danger', onclick: boot.learned ? async () => { await call('forgetLearned'); boot.learned = 0; flash = t('priv.forgotten'); render(); } : undefined })),
        flash ? block(note(flash)) : null,
      ],
    };
  },

  updates: () => updatesPage(),

  about() {
    const edition = t(boot.edition === 'native' ? 'about.native' : 'about.portable');
    const details = `Likhon ${boot.version} · ${boot.edition === 'native' ? 'Full' : 'Portable'} · ${boot.windows}`;
    const link = (ic: string, id: string, path: string): Card => ({ icon: ic, label: t('about.' + id), desc: t('about.' + id + 'P'), onclick: () => site(path), title: t('opensSite') });
    return {
      keys: [],
      rows: [
        h('div', { class: 'r brand', 'data-find': 'Likhon' },
          h('span', { class: 'mark' }, icon('quill', 30)),
          h('div', { class: 'lbl' }, h('div', { class: 'l' }, t('about.line', { version: boot.version })), h('div', { class: 'd' }, edition + ' · ' + boot.windows)),
          h('div', { class: 'ctl' }, h('button', {
            type: 'button', class: 'btn', onclick: (e: Event) => {
              call('copy', { text: details });
              (e.currentTarget as HTMLElement).lastChild!.textContent = t('copied');
            },
          }, icon('copy', 14), t('about.copy')))),
        h('p', { class: 'muted small', 'data-find': 'agreement' }, t('about.agreement', { date: boot.agreementDate })),
        cards([
          link('life-buoy', 'support', 'support'), link('graduation-cap', 'learn', 'learn'), link('message-circle', 'forum', 'forum'),
          link('scroll-text', 'license', 'legal'),
          link('file-text', 'licensingTerms', 'legal/licensing'), link('feather', 'notices', 'legal/notices'),
          { icon: 'code', label: t('about.credits'), desc: t('about.creditsP'), onclick: async () => dialog(t('about.credits'), await (await fetch('licenses.txt')).text()) },
          link('hand-heart', 'contributors', 'contributors'),
        ]),
      ],
    };
  },
};

// ---- Online: Updates, Library, Donate words (app/src/online.rs; only when the person clicks) ----------

/** A one-off message shown on the current page until the next page change. */
let flash = '';
type Release = { ok: boolean; version?: string; newer?: boolean; download?: string; error?: string; dict?: { version: string; words?: number | null; newer: boolean } | null };
let release: Release | 'checking' | null = null;
let dict: { ok: boolean; words?: number; error?: string } | 'loading' | null = null;

function updatesPage(): Page {
  const r = release;
  const status = r === 'checking' ? t('upd.checking') : !r ? t('upd.never') : !r.ok ? t('upd.failed', { error: r.error ?? '' })
    : !r.version ? t('upd.noRelease') : r.newer ? t('upd.newer', { version: num(r.version) }) : t('upd.latest');
  const d = dict;
  // A newer dictionary on the site (found by Check for updates): words people donated and reviewers approved.
  const newDict = r && r !== 'checking' && r.ok && r.dict?.newer ? r.dict : null;
  return {
    keys: [],
    rows: [
      row(t('upd.version', { version: num(boot.version) }), status, [
        r && r !== 'checking' && r.ok && r.newer ? button(t('upd.getIt'), { icon: 'download', onclick: () => site('download') }) : null,
        button(t('upd.check'), { icon: 'refresh-cw', kind: 'primary', onclick: r === 'checking' ? undefined : async () => {
          release = 'checking'; render();
          release = await call<Release>('checkUpdate'); render();
        } })]),
      row(t('upd.dictionaryH'), boot.dictionary ? t('upd.dictWords', { n: num(boot.dictionary.words.toLocaleString('en-IN')) }) : t('upd.dictMissing'),
        boot.dictionary ? h('span', { class: 'chip' }, boot.dictionary.license || 'MPL-2.0') : h('span', { class: 'chip warn' }, '!')),
      row(t('upd.dictH'), d === 'loading' ? t('lib.loading') : d ? (d.ok ? t('upd.dictDone', { n: num((d.words ?? 0).toLocaleString('en-IN')) }) : t('upd.dictFailed', { error: d.error ?? '' }))
        : newDict ? t('upd.dictNewer', { version: num(newDict.version), n: num((newDict.words ?? 0).toLocaleString('en-IN')) }) : t('upd.dictP'),
        button(t('upd.dictGet'), { icon: 'download', kind: newDict && !d ? 'primary' : undefined, onclick: d === 'loading' ? undefined : async () => {
          dict = 'loading'; render();
          dict = await call('dictDownload');
          if (dict && dict !== 'loading' && dict.ok) {
            boot.dictionary = { words: dict.words ?? 0, path: '', license: boot.dictionary?.license ?? '', snapshot: newDict?.version ?? boot.dictionary?.snapshot };
            if (release && release !== 'checking' && release.dict) release.dict.newer = false;
          }
          render();
        } })),
      block(note(t('upd.siteNote', { site: host() }), 'lock')),
      cards([{ icon: 'rss', label: t('upd.newsH'), desc: t('upd.newsP'), onclick: () => site('news'), title: t('opensSite') }]),
    ],
  };
}

type LibItem = { id: string; type: string; name: Record<Lang, string>; author?: string; license?: string; version?: string; size?: number;
  signed?: boolean; added?: string | null; can?: boolean; page?: string; file?: string; sha256?: string };
let lib: { ok: boolean; items?: LibItem[]; error?: string } | 'loading' | null = null;
let libAdded: Record<string, { type: string; version: string; name: Record<Lang, string> }> | null = null;
let libBusy = '';
/** The Library page's tab: one kind of item (fonts, themes, layouts; word lists and the rest are "other"). */
let libTab: 'fonts' | 'themes' | 'layouts' | 'other' = 'fonts';
const LIB_TABS = ['fonts', 'themes', 'layouts', 'other'] as const;
const libKind = (type: string) => (type === 'layout' ? 'layouts' : type === 'theme' ? 'themes' : type === 'font' ? 'fonts' : 'other');
const inLibTab = (type: string) => libKind(type) === libTab;

function libraryPage(): Page {
  if (libAdded === null) call('libraryAdded').then((v) => { libAdded = (v ?? {}) as typeof libAdded; render(); });
  const typeName = (ty: string) => { const s = t('lib.type.' + ty); return s.startsWith('app.') ? ty : s; };
  const added = Object.entries(libAdded ?? {}).filter(([, a]) => inLibTab(a.type));
  const remove = (id: string) => async () => {
    libBusy = id; render();
    await call('libraryRemove', { id });
    libAdded = null; libBusy = ''; if (lib && lib !== 'loading' && lib.items) lib.items.forEach((i) => { if (i.id === id) i.added = null; });
    // A removed layout or theme leaves the Layouts and Appearance lists.
    delete side.layouts; delete side.themes;
    render();
  };
  const L = lib;
  const all = L && L !== 'loading' && L.ok ? L.items ?? [] : [];
  const items = all.filter((i) => inLibTab(i.type));
  return {
    keys: [],
    rows: [
      block(h('div', { class: 'chips', role: 'tablist', 'aria-label': t('lib.tabsAria') }, LIB_TABS.map((k) =>
        h('button', { type: 'button', role: 'tab', 'aria-selected': String(k === libTab), onclick: () => { libTab = k; render(); } }, t('lib.tab.' + k))))),
      added.length
        ? h('div', { class: 'r wide', 'data-find': t('lib.added') }, h('div', { class: 'table lib' }, added.map(([id, a]) =>
          h('div', { class: 'tr' }, h('span', { class: 'td' }, a.name?.[lang()] ?? id, h('span', { class: 'chip' }, typeName(a.type))),
            h('span', { class: 'td' }, a.version ? 'v' + num(a.version) : ''),
            h('span', { class: 'td' }, button(t('lib.remove'), { kind: 'danger', onclick: libBusy ? undefined : remove(id) }))))))
        : !Object.keys(libAdded ?? {}).length
          ? block(h('div', { class: 'empty' }, icon('library', 28), h('b', {}, t('lib.none')), h('span', {}, t('lib.noneP'))))
          : block(h('div', { class: 'tempty' }, t('lib.tabEmpty'))),
      row(t('lib.browseH'), L === 'loading' ? t('lib.loading') : L && !L.ok ? t('lib.failed', { error: L.error ?? '' }) : L && !all.length ? t('lib.empty') : L && !items.length ? t('lib.tabEmpty') : t('lib.browseP'), [
        button(t('lib.findBtn'), { icon: 'library', onclick: () => site('library'), title: t('opensSite') }),
        button(t('lib.load'), { icon: 'refresh-cw', kind: 'primary', onclick: L === 'loading' ? undefined : async () => { lib = 'loading'; render(); lib = await call('libraryList'); render(); } })]),
      items.some((i) => !i.signed) ? block(note(t('lib.unsignedWarn'), 'warn')) : null,
      flash ? block(note(flash, 'warn')) : null,
      items.length ? h('div', { class: 'r wide', 'data-find': t('lib.browseH') }, h('div', { class: 'table lib' }, items.map((it) =>
        h('div', { class: 'tr' },
          h('span', { class: 'td' }, it.name[lang()] || it.id, h('span', { class: 'chip' }, typeName(it.type)), it.signed ? null : h('span', { class: 'chip warn' }, t('lib.unsigned')),
            h('span', { class: 'muted' }, ' ' + [it.author, it.license].filter(Boolean).join(' · '))),
          h('span', { class: 'td' }, it.version ? 'v' + num(it.version) : ''),
          h('span', { class: 'td' }, !it.can
            ? button(t('lib.onSite'), { onclick: it.page ? () => call('openUrl', { url: it.page! }) : undefined, title: t('opensSite') })
            : it.added
              ? button(t('lib.remove'), { kind: 'danger', onclick: libBusy ? undefined : remove(it.id) })
              : button(libBusy === it.id ? t('lib.loading') : t('lib.add'), { icon: 'download', kind: 'primary', onclick: libBusy ? undefined : async () => {
                libBusy = it.id; flash = ''; render();
                const r = await call<{ ok: boolean; error?: string }>('libraryAdd', { item: it as unknown as Json });
                libBusy = '';
                // An added layout or theme shows up in Layouts and Appearance (their lists are read once, then kept).
                if (r.ok) { it.added = it.version ?? ''; libAdded = null; delete side.layouts; delete side.themes; } else flash = t('lib.addFailed', { error: r.error ?? '' });
                render();
              } })))))) : null,
    ],
  };
}

/** Privacy › Donate words: the learned words as a review list; only the ticked ones are sent. */
async function donate() {
  const words = (await call<string[]>('learnedWords')) ?? [];
  if (!words.length) { dialog(t('priv.donateH'), t('priv.donateNone')); return; }
  const picked = new Set<string>();
  const send = button(t('priv.donateSend'), { kind: 'primary', onclick: () => undefined });
  send.disabled = true;
  const body = h('div', { class: 'donate' },
    h('p', {}, t('priv.donateDlg', { site: host() })),
    h('p', { class: 'notice-warn' }, t('priv.donateWarn')),
    h('p', { class: 'muted small' }, t('priv.donateFinal')),
    h('div', { class: 'words', lang: 'bn' }, words.slice(0, 500).map((w) => h('label', {},
      h('input', { type: 'checkbox', onchange: (e: Event) => { (e.target as HTMLInputElement).checked ? picked.add(w) : picked.delete(w); send.disabled = !picked.size; } }), w))));
  const close = dialog(t('priv.donateH'), body, [send]);
  send.addEventListener('click', async () => {
    send.disabled = true;
    const r = await call<{ ok: boolean; accepted?: number; error?: string }>('donate', { words: [...picked] });
    close();
    flash = r.ok ? t('priv.donateDone', { n: num(picked.size) }) : t('priv.donateFailed', { error: r.error ?? '' });
    render();
  });
}

// ---- My words, autocorrect, shortcuts (user.sqlite) ---------------------------------------------------

type Entry = { key: string; value: string; added: number };
const lists: Record<string, Entry[] | null> = {};
const listMsg: Record<string, string> = {};
const listFilter: Record<string, string> = {};

async function reloadList(list: string) {
  lists[list] = (await call<Entry[]>('userList', { list })) ?? [];
  render();
}

/** An editable list from user.sqlite: add a row, remove a row, import and export a plain text file. */
function userTable(list: 'words' | 'autocorrect' | 'shortcuts' | 'choices', c1: string, c2: string, empty: string, ph1: string, ph2: string) {
  if (!(list in lists)) {
    lists[list] = null;
    reloadList(list);
  }
  const rows = lists[list];
  const words = list === 'words';
  const q = (listFilter[list] ?? '').toLowerCase();
  const shown = (rows ?? []).filter((e) => !q || e.key.toLowerCase().includes(q) || e.value.toLowerCase().includes(q));
  const date = (s: number) => new Date(s * 1000).toLocaleDateString(lang() === 'bn' ? 'bn-BD' : 'en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
  const k = h('input', { type: 'text', placeholder: ph1, 'aria-label': c1, spellcheck: 'false', maxlength: 60 });
  const v = words ? null : h('input', { type: 'text', placeholder: ph2, 'aria-label': c2, spellcheck: 'false', maxlength: 500 });
  const add = async () => {
    const r = await call<{ ok?: boolean; error?: string }>('userAdd', { list, key: k.value, value: v?.value ?? '' });
    listMsg[list] = r?.error ? t('lists.err.' + r.error) : '';
    if (r?.ok) await reloadList(list); else render();
  };
  const enter = (e: KeyboardEvent) => { if (e.key === 'Enter') add(); };
  k.addEventListener('keydown', enter);
  v?.addEventListener('keydown', enter);
  return h('div', { class: 'table' },
    h('div', { class: 'th' }, h('span', {}, c1), h('span', {}, c2), h('span', {})),
    h('div', { class: 'tr add' }, k, v ?? h('span', { class: 'muted' }, t('lists.today')),
      h('span', { class: 'end' }, h('button', { type: 'button', class: 'icon-btn', 'aria-label': t('add'), title: t('add'), onclick: add }, icon('plus', 15)))),
    listMsg[list] ? h('div', { class: 'msg' }, icon('alert-triangle', 14), listMsg[list]) : null,
    rows === null ? h('div', { class: 'tempty' }, t('fonts.loading'))
      : shown.length ? h('div', { class: 'tbody' }, shown.slice(0, 500).map((e) => h('div', { class: 'tr' },
        h('span', { class: list === 'shortcuts' || list === 'choices' ? 'mono' : '', lang: list === 'shortcuts' || list === 'choices' ? undefined : 'bn' }, e.key),
        h('span', { class: 'val', lang: words ? undefined : 'bn' }, words ? date(e.added) : e.value),
        h('span', { class: 'end' }, h('button', {
          type: 'button', class: 'icon-btn', 'aria-label': t('lists.remove', { key: e.key }), title: t('lists.remove', { key: e.key }),
          onclick: async () => { await call('userRemove', { list, key: e.key }); await reloadList(list); },
        }, icon('trash-2', 14))))))
        : h('div', { class: 'tempty' }, rows.length ? t('findNone', { q: listFilter[list] ?? '' }) : empty),
    h('div', { class: 'tf' },
      (rows?.length ?? 0) > 8 ? h('label', { class: 'search mini' }, icon('search', 13), h('input', {
        type: 'search', placeholder: t('lists.filter'), 'aria-label': t('lists.filter'), value: listFilter[list] ?? '',
        oninput: (e: Event) => { listFilter[list] = (e.target as HTMLInputElement).value; render(); (document.querySelector('.search.mini input') as HTMLInputElement | null)?.focus(); },
      })) : null,
      h('span', { class: 'muted small' }, t('lists.count', { n: num(rows?.length ?? 0) })),
      h('span', { class: 'grow' }),
      h('button', {
        type: 'button', class: 'link-btn', onclick: async () => {
          const r = await call<{ added: number; skipped: number } | null>('userImport', { list });
          if (r) { listMsg[list] = t('lists.imported', { added: num(r.added), skipped: num(r.skipped) }); await reloadList(list); }
        },
      }, icon('file-up', 14), t('import')),
      h('button', { type: 'button', class: 'link-btn', disabled: !rows?.length, onclick: () => call('userExport', { list }) }, icon('file-down', 14), t('export'))));
}

// ---- The switch key (General) ---------------------------------------------------------------------

/** The hotkey setting being picked (hotkeys.toggle, hotkeys.next_layout), or null. */
let capturing: string | null = null;
let hotkeyWarn: string | null = null;
let warnFor: string | null = null;

function startCapture(key = 'hotkeys.toggle') {
  capturing = key;
  hotkeyWarn = null;
  call('capture', { on: true }); // the agent leaves the keyboard alone while a key is being picked
  render();
}

function stopCapture() {
  capturing = null;
  call('capture', { on: false });
  render();
}

const KEY_NAMES: Record<number, string> = { 32: 'Space', 9: 'Tab', 13: 'Enter', 192: '`', 189: '-', 187: '=', 219: '[', 221: ']', 220: '\\', 186: ';', 222: "'", 188: ',', 190: '.', 191: '/', 45: 'Insert', 46: 'Delete', 36: 'Home', 35: 'End', 33: 'Page Up', 34: 'Page Down', 19: 'Pause', 145: 'Scroll Lock' };
document.addEventListener('keydown', async (e) => {
  if (!capturing) return;
  e.preventDefault();
  e.stopPropagation();
  if (e.key === 'Escape') return stopCapture();
  const vk = e.keyCode;
  if ([16, 17, 18, 91, 92, 93].includes(vk)) return; // modifiers alone
  const fkey = vk >= 112 && vk <= 135;
  const target = capturing;
  warnFor = target;
  if (!fkey && !e.ctrlKey && !e.altKey) {
    hotkeyWarn = t('hotkeys.needKey');
    capturing = null;
    call('capture', { on: false });
    return render();
  }
  const name = fkey ? 'F' + (vk - 111) : (vk >= 48 && vk <= 90) ? String.fromCharCode(vk) : (KEY_NAMES[vk] ?? e.key.toUpperCase());
  const label = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', name].filter(Boolean).join(' + ');
  const hk: Hotkey = { vk, ctrl: e.ctrlKey, alt: e.altKey, shift: e.shiftKey, label };
  capturing = null;
  // The two Likhon keys must differ; another program holding the key is a warning only.
  const other = get<Hotkey | null>(target === 'hotkeys.toggle' ? 'hotkeys.next_layout' : 'hotkeys.toggle');
  if (other && other.vk === hk.vk && other.ctrl === hk.ctrl && other.alt === hk.alt && other.shift === hk.shift) {
    hotkeyWarn = t('hk.same', { key: label });
    await call('capture', { on: false });
    return render();
  }
  const free = await call<boolean>('hotkeyFree', hk);
  hotkeyWarn = free ? null : t('hotkeys.conflict', { key: label });
  await call('capture', { on: false });
  await set(target!, hk);
}, true);

// ---- Apps that keep the switch key (General) ------------------------------------------------------

let picker: { apps: App[]; q: string } | null = null;

function passList() {
  const list = get<App[]>('apprules.passthrough');
  const defaults = (boot.defaults['apprules.passthrough'] as App[]).map((a) => a.exe.toLowerCase());
  const save = (next: App[]) => set('apprules.passthrough', next as unknown as Json);
  const title = t('general.passH').replace('F12', get<Hotkey>('hotkeys.toggle').label);
  return h('div', { class: 'r wide', 'data-find': title },
    h('div', { class: 'lbl' }, h('div', { class: 'l' }, title), h('div', { class: 'd' }, t('general.passP'))),
    h('div', { class: 'table' },
      h('div', { class: 'th' }, h('span', {}, t('general.passApp')), h('span', {}, t('general.passFile')), h('span', {})),
      list.length ? list.map((a) => h('div', { class: 'tr' },
        h('span', {}, a.name, defaults.includes(a.exe.toLowerCase()) ? h('span', { class: 'chip' }, t('general.passDefault')) : null),
        h('span', { class: 'mono' }, a.exe),
        h('span', { class: 'end' }, h('button', { class: 'icon-btn', type: 'button', 'aria-label': t('general.passRemove', { name: a.name }), title: t('general.passRemove', { name: a.name }), onclick: () => save(list.filter((x) => x !== a)) }, icon('trash-2', 14)))))
        : h('div', { class: 'tempty' }, t('general.passEmpty')),
      h('div', { class: 'tf' },
        h('button', { type: 'button', class: 'link-btn', 'aria-expanded': String(!!picker), onclick: openPicker }, icon('plus', 14), t('general.passAdd')),
        h('button', { type: 'button', class: 'link-btn', onclick: async () => { const a = await call<App | null>('pickExe'); if (a && !list.some((x) => x.exe.toLowerCase() === a.exe.toLowerCase())) save([...list, a]); } }, icon('folder-open', 14), t('general.passBrowse'))),
      picker ? appPicker(list, save) : null));
}

async function openPicker() {
  if (picker) { picker = null; return render(); }
  picker = { apps: [], q: '' };
  render();
  picker.apps = (await call<App[]>('runningApps')) ?? [];
  render();
  (document.querySelector('.pop input') as HTMLInputElement | null)?.focus();
}

function appPicker(list: App[], save: (next: App[]) => void) {
  const p = picker!;
  const have = new Set(list.map((a) => a.exe.toLowerCase()));
  const add = (a: App) => { picker = null; if (!have.has(a.exe.toLowerCase())) save([...list, a]); else render(); };
  const items = () => p.apps.filter((a) => !have.has(a.exe.toLowerCase()) && (a.name + ' ' + a.exe).toLowerCase().includes(p.q.toLowerCase()));
  const listEl = h('div', { class: 'list' });
  const fill = () => {
    const found = items();
    listEl.replaceChildren(...(found.length ? found.map((a) => h('button', { class: 'item', type: 'button', onclick: () => add(a) }, h('span', {}, a.name), h('span', { class: 'mono' }, a.exe)))
      : [h('div', { class: 'cap' }, t('general.passNone'))]));
  };
  fill();
  return h('div', { class: 'pop', role: 'dialog', 'aria-label': t('general.passAdd') },
    h('label', { class: 'search' }, icon('search', 14),
      h('input', { type: 'text', placeholder: t('general.passSearch'), 'aria-label': t('general.passSearch'), value: p.q, oninput: (e: Event) => { p.q = (e.target as HTMLInputElement).value; fill(); } })),
    h('div', { class: 'cap' }, t('general.passRunning')),
    listEl);
}

// ---- Habits from another keyboard (presets, Desktop/CLAUDE.md 7: named by habit, never by brand) ------------------

const PRESETS: Record<string, Record<string, Json>> = {
  default: { 'hotkeys.toggle': { vk: 0x7B, ctrl: false, alt: false, shift: false, label: 'F12' }, layout: 'phonetic', 'typing.bangla_digits': false, 'typing.dari': true },
  ctrlspace: { 'hotkeys.toggle': { vk: 0x20, ctrl: true, alt: false, shift: false, label: 'Ctrl + Space' }, layout: 'phonetic' },
  fixed: { 'hotkeys.toggle': { vk: 0x42, ctrl: true, alt: true, shift: false, label: 'Ctrl + Alt + B' }, layout: 'jatiya', 'typing.dari': true },
  digits: { 'typing.bangla_digits': true },
};

/** The preset whose settings all match, or "custom". */
function presetNow(): string {
  const same = (a: Json, b: Json) => JSON.stringify(a) === JSON.stringify(b);
  return Object.keys(PRESETS).find((k) => Object.entries(PRESETS[k]).every(([key, v]) => same(get(key), v))) ?? 'custom';
}

async function applyPreset(k: string) {
  for (const [key, v] of Object.entries(PRESETS[k] ?? {})) await set(key, v, false);
  render();
}

function presetSelect(label: string) {
  const cur = presetNow();
  const items = Object.keys(PRESETS).map((v) => ({ v, label: t('preset.' + v) }));
  if (cur === 'custom') items.unshift({ v: 'custom', label: t('preset.custom') });
  return select(label, cur, items, (v) => { if (v !== 'custom') applyPreset(v); });
}

// ---- Hotkeys (archive S05) ------------------------------------------------------------------------

/** The switch key row, with Change (General and Hotkeys). */
const toggleRow = (label: string, desc: string) => keyRow('hotkeys.toggle', label, desc);

/** A hotkey row: the keys, Change (press the new keys), and Clear for keys that may be unset. */
function keyRow(key: string, label: string, desc: string | null, clearable = false) {
  const cap = capturing === key;
  const cur = get<Hotkey | null>(key);
  const warn = warnFor === key && hotkeyWarn && !cap;
  return h('div', { class: 'r' + (cap ? ' capture' : '') + (warn ? ' warn' : ''), 'data-find': label },
    h('div', { class: 'lbl' }, h('div', { class: 'l' }, label), desc ? h('div', { class: 'd' }, desc) : null),
    h('div', { class: 'ctl keys' }, cap ? h('kbd', { class: 'wait' }, t('hotkeys.press')) : cur ? keys(cur.label) : h('span', { class: 'muted' }, t('notSet')),
      h('button', { type: 'button', class: 'link-btn', onclick: () => (cap ? stopCapture() : startCapture(key)) }, cap ? t('cancel') : t('change')),
      clearable && cur && !cap ? h('button', { type: 'button', class: 'link-btn', onclick: () => set(key, null) }, t('hk.clear')) : null),
    warn ? h('div', { class: 'msg' }, icon('alert-triangle', 15), hotkeyWarn) : null);
}

function hotkeysPage(): Page {
  // The quick hotkeys (app/src/quick.rs), the designs' keys (v2 §11, archive S05); Ctrl with the left Alt key.
  const quick: [string, string][] = [['convert', 'Ctrl + Alt + U'], ['undo', 'Ctrl + Alt + Z'], ['addWord', 'Ctrl + Alt + W'],
    ['quickTools', 'Ctrl + Alt + T'], ['howType', 'Ctrl + Alt + H'], ['symbols', 'Ctrl + Alt + S']];
  return {
    keys: ['hotkeys.toggle', 'hotkeys.next_layout'],
    rows: [
      toggleRow(t('hk.toggle'), t('hk.passNote')),
      keyRow('hotkeys.next_layout', t('hk.nextLayout'), t('hk.nextLayoutP'), true),
      ...quick.map(([id, k]) => row(t('hk.' + id), t('hk.' + id + 'P'), h('span', { class: 'ctl keys' }, keys(k)))),
      row(t('hk.osk'), null, h('span', { class: 'ctl keys' }, keys('Ctrl + Alt + K'))),
      row(t('hk.kill'), t('hk.killP'), h('span', { class: 'ctl keys' }, keys('Ctrl + Alt + F12'))),
      row(t('hk.presetsH'), t('general.habitsP'), presetSelect(t('hk.presetsH'))),
    ],
  };
}

// ---- Fonts (archive S16) --------------------------------------------------------------------------

let fonts: string[] | null = null;
let fontPick: string | null = null;
/** The face Likhon made the Windows default (undefined until asked), and the last change's message. */
let fontNow: string | null | undefined;
let fontMsg = '';
let fontBusy = false;
let winKbMsg = '';
let winKbBusy = false;
let winKbOk = false;

async function changeWinKb(on: boolean) {
  winKbBusy = true; winKbMsg = ''; render();
  const r = await call<{ ok: boolean; added: boolean; code?: number; noFiles?: boolean; error?: string }>('windowsKeyboard', { on });
  winKbBusy = false;
  boot.windowsKeyboard = r.added;
  if (r.ok) boot.windowsKeyboardStale = false;
  winKbOk = r.ok;
  if (r.ok) winKbMsg = t(on ? 'general.winKbAdded' : 'general.winKbRemoved');
  else winKbMsg = r.error === 'cancelled' ? t('fonts.cancelled') : r.noFiles ? t('general.winKbNoFiles') : t('general.winKbFailed', { code: String(r.code ?? '') });
  render();
}

/** Puts Windows' own Bangla font links back (the font changer was removed, owner decision 2026-10-24). */
async function restoreDefault() {
  fontBusy = true; fontMsg = ''; render();
  const r = await call<{ ok: boolean; face?: string | null; code?: number; error?: string }>('fontDefault', { face: null });
  fontBusy = false;
  if (r.ok) { fontNow = r.face ?? null; fontMsg = t('fonts.restored'); }
  else fontMsg = r.error === 'cancelled' ? t('fonts.cancelled') : t('fonts.defaultFailed', { code: String(r.code ?? '') });
  render();
}

function fontsPage(): Page {
  if (fontNow === undefined) {
    fontNow = null;
    call<string | null>('fontDefaultNow').then((f) => { fontNow = f; if (page === 'fonts') render(); });
  }
  if (fonts === null) {
    call<string[]>('bnFonts').then((list) => { fonts = list ?? []; if (page === 'fonts') render(); });
  }
  const sample = t('fonts.sample');
  const face = (el: HTMLElement, name: string) => { el.style.fontFamily = `"${name.replace(/"/g, '')}"`; return el; };
  const list = fonts?.length
    ? h('div', { class: 'fontlist', role: 'listbox', 'aria-label': t('fonts.installedH') }, fonts.map((f) =>
      h('button', { type: 'button', role: 'option', 'aria-selected': String(f === fontPick), class: 'font', onclick: () => { fontPick = f === fontPick ? null : f; render(); } },
        h('span', { class: 'fn' }, f), face(h('span', { class: 'fs', lang: 'bn' }, sample), f))))
    : h('div', { class: 'tempty' }, t(fonts === null ? 'fonts.loading' : 'fonts.none'));
  const preview = fontPick ? h('div', { class: 'fontpreview' },
    h('b', {}, t('fonts.previewH', { name: fontPick })),
    ...[9, 11, 14, 20].map((pt) => {
      const line = face(h('div', { lang: 'bn' }, h('span', { class: 'pt' }, t('fonts.size', { n: num(pt) })), sample), fontPick!);
      line.style.fontSize = (pt * 4) / 3 + 'px';
      return line;
    })) : null;
  return {
    keys: [],
    rows: [
      h('div', { class: 'r wide', 'data-find': t('fonts.installedH') },
        h('div', { class: 'lbl' }, h('div', { class: 'l' }, t('fonts.installedH'), fonts?.length ? h('span', { class: 'chip' }, t('fonts.count', { n: num(fonts.length) })) : null),
          h('div', { class: 'd' }, t('fonts.installedP'))),
        list, preview),
      row(t('fonts.libraryH'), t('fonts.libraryP'), button(t('fonts.libraryBtn'), { icon: 'library', onclick: () => go('library') })),
      // The Default Bangla font changer is gone (owner decision 2026-10-24: most of Windows 11 ignores font links).
      // Only when an earlier copy changed it: Restore (app/src/fontdefault.rs, elevated) puts Windows' own setting back.
      fontNow || fontMsg ? stacked(row(t('fonts.defaultH'), [fontNow ? t('fonts.restoreP', { font: fontNow }) : null, fontMsg ? h('div', { class: 'msg' + (fontNow ? '' : ' ok') }, fontMsg) : null],
        fontNow ? button(t('fonts.restoreBtn'), { onclick: fontBusy ? undefined : restoreDefault }) : null)) : null,
    ],
  };
}

// ---- Learn (archive S14) --------------------------------------------------------------------------

// Practice lines, typed in phonetic (Bangla sample text, not interface text).
const PRACTICE = ['amar sOnar bangla', 'ami tOmay bhalobasi', 'ami banglay gan gai', 'bangladesh', 'shikkha'];
// Cheat sheet keys; what they give comes from the engine, so the sheet always matches typing.
const CHEAT = ['o', 'a', 'i', 'I', 'u', 'U', 'rri', 'e', 'OI', 'O', 'OU', 'ng', 'kh', 'gh', 'ch', 'th', 'sh', 'Sh', 'kkh', 'gg'];
let practiceAt = 0;
let practiceTyped = '';
const engineSays: Record<string, string> = {};
let asking = false;

function learnPage(): Page {
  const missing = ['amar', ...PRACTICE, ...CHEAT].filter((k) => !(k in engineSays));
  if (missing.length && !asking) {
    asking = true;
    Promise.all(missing.map(async (k) => { engineSays[k] = String(await call('phonetic', { text: k })); }))
      .then(() => { asking = false; if (page === 'learn') render(); });
  }
  const toggle = get<Hotkey>('hotkeys.toggle');
  const layout = get<string>('layout');
  const goal = engineSays[PRACTICE[practiceAt]] ?? '';
  const line = h('div', { class: 'line', lang: 'bn' });
  const typedOut = h('div', { class: 'typed', lang: 'bn' });
  const paint = (mine: string) => {
    let ok = 0;
    while (ok < mine.length && ok < goal.length && mine[ok] === goal[ok]) ok++;
    const wrong = mine.length > ok;
    line.replaceChildren(h('span', { class: 'done' }, goal.slice(0, ok)), wrong ? h('span', { class: 'bad' }, goal.slice(ok, ok + 1)) : '', goal.slice(ok + (wrong ? 1 : 0)));
    typedOut.textContent = mine && mine === goal ? mine + '  ✓ ' + t('learn.done') : mine;
  };
  // With Bangla on, the agent already types Bangla into the box; otherwise the letters are converted here.
  const show = async () => paint(/[ঀ-৿]/.test(practiceTyped) ? practiceTyped : practiceTyped ? String(await call('phonetic', { text: practiceTyped })) : '');
  const input = h('input', {
    type: 'text', value: practiceTyped, spellcheck: 'false', autocomplete: 'off', 'aria-label': t('learn.yourTyping'),
    oninput: (e: Event) => { practiceTyped = (e.target as HTMLInputElement).value; show(); },
  });
  show();
  const step = (n: number, id: string, body: Child[]) => h('div', { class: 'step' }, h('span', { class: 'n' }, num(n)), h('b', {}, t('learn.' + id + 'h')), h('span', {}, ...body));
  const fill = (id: string, vars: Record<string, Node>) => {
    const out: Child[] = [];
    const s = t(id);
    let last = 0;
    for (const m of s.matchAll(/\{(\w+)\}/g)) { out.push(s.slice(last, m.index), vars[m[1]]); last = (m.index ?? 0) + m[0].length; }
    out.push(s.slice(last));
    return out;
  };
  return {
    keys: [],
    rows: [
      block(h('div', { class: 'steps' },
        step(1, 's1', fill('learn.s1p', { key: h('kbd', {}, toggle.label) })),
        step(2, 's2', fill('learn.s2p', { from: h('span', { class: 'mono' }, 'amar'), to: h('span', { lang: 'bn' }, engineSays.amar ?? '') })),
        step(3, 's3', [t('learn.s3p')]))),
      layout !== 'phonetic' ? block(note(t('learn.fixedNote', { layout: t('layouts.' + layout) }))) : null,
      h('div', { class: 'r wide', 'data-find': t('learn.cheatH') },
        h('div', { class: 'lbl' }, h('div', { class: 'l' }, t('learn.cheatH')), h('div', { class: 'd' }, t('learn.cheatP'))),
        h('div', { class: 'cheat' }, CHEAT.map((k) => h('div', {}, h('span', { class: 'mono' }, k), h('span', { lang: 'bn' }, engineSays[k] ?? ''))))),
      h('div', { class: 'r wide practice', 'data-find': t('learn.practiceH') },
        h('div', { class: 'lbl' }, h('div', { class: 'l' }, t('learn.practiceH')), h('div', { class: 'd' }, t('learn.practiceP'))),
        line, input, typedOut,
        h('div', { class: 'acts' }, button(t('learn.next'), { icon: 'arrow-right', onclick: () => { practiceAt = (practiceAt + 1) % PRACTICE.length; practiceTyped = ''; render(); } }))),
      row(t('learn.more'), null, button(t('about.learn'), { icon: 'graduation-cap', onclick: () => site('learn'), title: t('opensSite') })),
    ],
  };
}

// ---- Themes (Appearance) --------------------------------------------------------------------------

type Theme = { id: string; name: { en: string; bn?: string }; light: Record<string, string>; dark: Record<string, string> };
// The built-in themes: assets/themes/*.json, which the top bar reads too (app/build.rs).
let THEMES: Theme[] = [];

/** A built-in theme's name comes from the text library; a theme from a file carries its own. */
const themeName = (th: Theme) => (text.en['app.set.appearance.t.' + th.id] ? t('appearance.t.' + th.id) : (lang() === 'bn' && th.name.bn) || th.name.en);

function effectiveDark(): boolean {
  const c = get<string>('color');
  return c === 'dark' || (c === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);
}

// ---- Added from a file (v2 §12): layouts and themes only, JSON only, with a not-reviewed warning ----------------

type SideItem = { id: string; name: { en: string; bn?: string }; light?: Record<string, string>; dark?: Record<string, string> };
const side: Record<string, SideItem[] | null> = {};
const sideMsg: Record<string, string> = {};

function sideloaded(kind: 'layouts' | 'themes'): SideItem[] {
  if (!(kind in side)) {
    side[kind] = null;
    call<SideItem[]>('sideloaded', { kind }).then((v) => { side[kind] = v ?? []; render(); });
  }
  return side[kind] ?? [];
}

/** Added themes, with "@token" colours read from the page's own token variables. */
function sideThemes(): Theme[] {
  const css = getComputedStyle(document.documentElement);
  const fix = (c: Record<string, string> | undefined) => Object.fromEntries(Object.entries(c ?? {}).map(([k, v]) => [k, v.startsWith('@') ? css.getPropertyValue('--c-' + v.slice(1)).trim() : v]));
  return sideloaded('themes').map((x) => ({ id: x.id, name: x.name, light: fix(x.light), dark: fix(x.dark) }));
}

function addFromFile(kind: 'layouts' | 'themes') {
  // Warn first: the file has not been reviewed or signed (the library's items are).
  dialog(t('side.warnH'), t('side.warnP'), [button(t('side.choose'), {
    kind: 'primary',
    onclick: async () => {
      document.querySelector('.scrim')?.remove();
      const r = await call<{ id?: string; name?: string; error?: string } | null>('sideloadAdd', { kind });
      if (!r) return;
      sideMsg[kind] = r.error ? t('side.refused', { why: r.error }) : t('side.added', { name: r.name ?? '' });
      delete side[kind];
      render();
    },
  })]);
}

async function removeSideloaded(kind: 'layouts' | 'themes', id: string, setting: string) {
  await call('sideloadRemove', { kind, id });
  if (get<string | null>(setting) === id) await reset([setting]);
  delete side[kind];
  sideMsg[kind] = '';
  render();
}

/** Theme ids saved by earlier builds (same rule as canonical() in app/src/topbar.rs). */
const OLD_THEME_IDS: Record<string, string> = { 'default-light': 'default', 'default-dark': 'default', 'night-green': 'green', 'contrast-light': 'contrast', 'avro-aero7-dark': 'aero', 'avro-iavro': 'maroon' };
const canonicalTheme = (id: string) => OLD_THEME_IDS[id] ?? id.replace(/^avro-/, '');

/** The chosen theme, or the default one. Its colours come from `colors()`, in the version Light or dark asks for. */
function activeTheme(): Theme {
  const id = get<string | null>('appearance.theme') ?? 'default';
  return [...THEMES, ...sideThemes()].find((x) => x.id === canonicalTheme(id)) ?? THEMES[0];
}
const colors = (th: Theme) => (effectiveDark() ? th.dark : th.light);

// ---- Tools ----------------------------------------------------------------------------------------

let tool: string | null = null;
const toolText: Record<string, string> = {};
let chartLayout = 'probhat';
let chartLevel: 'normal' | 'shift' | 'altgr' = 'normal';
let symCat = 0;
let convDir: 'toBijoy' | 'fromBijoy' | 'toProshika' | 'fromProshika' = 'toBijoy';

/** A tool with text in and text out; the work is done by the engine in the exe. */
function ioTool(name: string, sample: string, run: (text: string) => Promise<string>, note: string) {
  if (!(name in toolText)) toolText[name] = sample;
  const out = h('div', { class: 'out', 'aria-live': 'polite', lang: 'bn' });
  const show = async () => { out.textContent = await run(toolText[name]); };
  show();
  return h('div', { class: 'tool' },
    backLink(),
    h('div', { class: 'io' },
      h('div', { class: 'col' },
        h('label', { class: 'cap', for: 'src' }, t('tools.in'),
          h('button', { class: 'link-btn', type: 'button', onclick: async () => { const s = await call<string>('paste'); if (s) { toolText[name] = s; render(); } } }, icon('paste', 13), t('paste'))),
        h('textarea', { id: 'src', spellcheck: 'false', oninput: (e: Event) => { toolText[name] = (e.target as HTMLTextAreaElement).value; show(); } }, toolText[name])),
      h('div', { class: 'col' },
        h('span', { class: 'cap' }, t('tools.out'),
          h('button', { class: 'link-btn', type: 'button', onclick: (e: Event) => { call('copy', { text: out.textContent ?? '' }); (e.currentTarget as HTMLElement).lastChild!.textContent = t('copied'); } }, icon('copy', 13), t('copy'))),
        out)),
    h('p', { class: 'd' }, note));
}

const backLink = () => h('button', { type: 'button', class: 'link-btn back', onclick: () => { tool = null; render(); } }, icon('arrow-left', 14), t('tools.all'));
const runTool = <T extends Json>(name: string, text: string) => call<T>('tool', { name, text });

// Symbols for the palette (characters, not interface text). Click one to copy it.
const SYMBOLS: [string, string][] = [
  ['sym.cat.bangla', '। ॥ ৳ ঃ ং ঁ ্ ঽ ৺ ৎ ৴ ৵ ৶ ৷ ৸ ৹'],
  ['sym.cat.money', '৳ $ € £ ₹ ¥ ¢ ₽ ₩ ₺'],
  ['sym.cat.numbers', '½ ⅓ ⅔ ¼ ¾ ⅕ ⅛ % ‰ ° № ²  ³'],
  ['sym.cat.math', '± × ÷ = ≠ ≈ ≤ ≥ < > ∞ √ ∑ π'],
  ['sym.cat.punct', '“ ” ‘ ’ « » — – … • · § ¶ © ® ™'],
  ['sym.cat.arrows', '← → ↑ ↓ ↔ ⇐ ⇒ ✓ ✗ ★ ☆ ♥'],
];

function symbolPalette() {
  const chars = SYMBOLS[symCat][1].split(' ').filter(Boolean);
  const copied = h('span', { class: 'muted small', 'aria-live': 'polite' });
  return h('div', { class: 'tool' },
    backLink(),
    h('div', { class: 'chips', role: 'tablist' }, SYMBOLS.map(([id], i) => h('button', { type: 'button', role: 'tab', 'aria-selected': String(i === symCat), onclick: () => { symCat = i; render(); } }, t(id)))),
    h('div', { class: 'symgrid' }, chars.map((c) => h('button', {
      type: 'button', class: 'sym', title: t('sym.copy', { c }), 'aria-label': t('sym.copy', { c }),
      onclick: () => { call('copy', { text: c }); copied.textContent = t('sym.copied', { c }); },
    }, c))),
    copied,
    h('p', { class: 'd' }, t('sym.note')));
}

/** The layout viewer: the keyboard with what each key types, for the chosen level. */
function layoutViewer() {
  const box = h('div', { class: 'chart', lang: 'bn' }, h('div', { class: 'tempty' }, t('fonts.loading')));
  call<{ keys: Record<string, Record<string, string>>; name: { en: string }; author: string } | null>('layoutChart', { id: chartLayout }).then((l) => {
    if (!l) return;
    const rows = ['`1234567890-=', 'qwertyuiop[]\\', "asdfghjkl;'", 'zxcvbnm,./'];
    const show = (ch: string) => (/^[া-ৌৗঁ-ঃ্]$/.test(ch) ? '◌' + ch : ch); // vowel signs on a dotted circle
    box.replaceChildren(...rows.map((r, i) => {
      const line = h('div', { class: 'krow' }, r.split('').map((k) => {
        const v = l.keys[k] ?? {};
        return h('div', { class: 'key' }, h('span', { class: 'lat' }, k.toUpperCase()), h('span', { class: 'kch' }, show(v[chartLevel] ?? '')),
          chartLevel === 'normal' && v.shift ? h('span', { class: 'sh' }, show(v.shift)) : null);
      }));
      line.style.paddingLeft = [0, 22, 32, 48][i] + 'px';
      return line;
    }), h('p', { class: 'd' }, t('viewer.by', { author: l.author || '' })));
  });
  // The built-in fixed layouts, then the ones added from the library or a file (input methods have no key chart).
  const choices = [...['probhat', 'jatiya'].map((v) => ({ v, label: t('layouts.' + v) })),
    ...sideloaded('layouts').filter((x) => (x as { kind?: string }).kind !== 'input-method').map((x) => ({ v: x.id, label: (lang() === 'bn' && x.name.bn) || x.name.en }))];
  return h('div', { class: 'tool' },
    backLink(),
    h('div', { class: 'toolbar' },
      select(t('layouts.listAria'), chartLayout, choices, (v) => { chartLayout = v; render(); }),
      seg(t('viewer.level'), chartLevel, [{ v: 'normal', label: t('viewer.normal') }, { v: 'shift', label: 'Shift' }, { v: 'altgr', label: 'AltGr' }], (v) => { chartLevel = v; render(); }),
      h('button', { type: 'button', class: 'btn', onclick: () => window.print() }, icon('file-down', 14), t('viewer.print'))),
    box);
}

function toolsPage(): Page {
  const views: Record<string, () => HTMLElement> = {
    taka: () => ioTool('taka', '১২৫০', async (s) => {
      const r = await runTool<{ words: string; amount: string } | null>('taka', s);
      return r ? r.words + '\n\n' + r.amount : t('tools.takaBad');
    }, t('tools.takaNote')),
    clean: () => ioTool('clean', t('tools.cleanSample'), async (s) => String((await runTool('clean', s)) ?? ''), t('tools.cleanNote')),
    num: () => ioTool('num', t('tools.numSample'), async (s) => String((await runTool('num', s)) ?? ''), t('tools.numNote')),
    how: () => ioTool('how', 'বাংলাদেশ', async (s) => {
      const words = s.split(/\s+/).filter(Boolean).slice(0, 30);
      const lines = await Promise.all(words.map(async (w) => w + '  →  ' + ((await runTool<string | null>('how', w)) ?? t('tools.howNone'))));
      return lines.join('\n');
    }, t('tools.howNote')),
    conv: () => {
      // A sample for each direction: the same sentence in Unicode, Bijoy and Proshika.
      const sample = { toBijoy: 'আমি বাংলায় গান গাই।', toProshika: 'আমি বাংলায় গান গাই।', fromBijoy: 'Avwg evsjvq Mvb MvB|', fromProshika: 'B¢j h¡wm¡u N¡e N¡Cz' }[convDir];
      const el = ioTool('conv-' + convDir, sample,
        async (s) => String((await runTool(convDir, s)) ?? ''), t('tools.convNote'));
      el.insertBefore(h('div', { class: 'toolbar' }, seg(t('tools.convDir'), convDir,
        [{ v: 'toBijoy', label: t('tools.convTo') }, { v: 'fromBijoy', label: t('tools.convFrom') },
          { v: 'toProshika', label: t('tools.convToProshika') }, { v: 'fromProshika', label: t('tools.convFromProshika') }], (v) => { convDir = v; render(); })), el.children[1]);
      return el;
    },
    sym: symbolPalette,
    viewer: layoutViewer,
  };
  if (tool && views[tool]) return { keys: [], rows: [h('div', { class: 'r wide' }, views[tool]())] };
  // The on-screen keyboard is a native window of the agent (app/src/osk.rs).
  const c = (ic: string, id: string, ready = false): Card => ({ icon: ic, label: t('tools.' + id), desc: t('tools.' + id + 'P'), later: !ready, onclick: ready ? () => { tool = id; render(); } : undefined });
  return {
    keys: [],
    rows: [cards([c('unicode-convert', 'conv', true), c('brush-cleaning', 'clean', true), c('taka-sign', 'taka', true), c('bangla-digits', 'num', true),
      c('how-to-type', 'how', true), c('omega', 'sym', true),
      { icon: 'keyboard', label: t('tools.osk'), desc: t('tools.oskP'), onclick: () => call('openOsk') }, c('layout', 'viewer', true)])],
  };
}

// ---- Shell: title bar, sidebar, finder -----------------------------------------------------------

function go(p: string, find?: string) {
  page = p;
  flash = '';
  query = '';
  render();
  const main = document.getElementById('main')!;
  main.scrollTop = 0;
  if (find) {
    const hit = [...main.querySelectorAll<HTMLElement>('[data-find]')].find((el) => el.dataset.find === find);
    hit?.scrollIntoView({ block: 'center' });
    hit?.classList.add('hit');
  }
}

/** Every row label on every page, for "Find a setting". */
function findAll(q: string): { page: string; label: string }[] {
  const needle = q.trim().toLowerCase();
  const out: { page: string; label: string }[] = [];
  const savedTool = tool;
  tool = null;
  for (const [p] of NAV) {
    const title = t('nav.' + p);
    if (title.toLowerCase().includes(needle)) out.push({ page: p, label: '' });
    const holder = h('div', {}, ...PAGES[p]().rows);
    for (const el of holder.querySelectorAll<HTMLElement>('[data-find]')) {
      const label = el.dataset.find ?? '';
      if (label && label.toLowerCase().includes(needle)) out.push({ page: p, label });
    }
  }
  tool = savedTool;
  return out;
}

function renderTitlebar() {
  const max = maximized ? t('restoreWin') : t('maximize');
  document.getElementById('titlebar')!.replaceChildren(
    icon('quill', 16),
    h('span', { class: 'title' }, t('window')),
    h('div', { class: 'wbtns' },
      h('button', { type: 'button', 'aria-label': t('minimize'), title: t('minimize'), onclick: () => call('win', { do: 'minimize' }) }, icon('minus', 14)),
      h('button', { type: 'button', 'aria-label': max, title: max, onclick: () => call('win', { do: 'maximize' }) }, icon(maximized ? 'copy' : 'square', 13)),
      h('button', { type: 'button', class: 'x', 'aria-label': t('closeWin'), title: t('closeWin'), onclick: () => call('win', { do: 'close' }) }, icon('x', 14))));
}

function renderShell() {
  renderTitlebar();

  const nav = document.getElementById('nav')!;
  nav.setAttribute('aria-label', t('navAria'));
  const search = h('label', { class: 'search' }, icon('search', 14),
    h('input', {
      type: 'search', placeholder: t('find'), 'aria-label': t('find'), value: query,
      oninput: (e: Event) => { query = (e.target as HTMLInputElement).value; renderNavList(); },
      onkeydown: (e: KeyboardEvent) => { if (e.key === 'Escape') { query = ''; renderShell(); } },
    }));
  nav.replaceChildren(search, h('div', { id: 'navlist', class: 'navlist' }));
  renderNavList();
}

function renderNavList() {
  const list = document.getElementById('navlist')!;
  if (!query.trim()) {
    list.replaceChildren(...NAV.map(([p, ic]) => h('a', { 'aria-current': p === page ? 'page' : undefined, href: '#', onclick: (e: Event) => { e.preventDefault(); go(p); } }, icon(ic, 18), t('nav.' + p))));
    return;
  }
  const hits = findAll(query);
  list.replaceChildren(...(hits.length ? hits.map((x) => h('a', { class: 'hitrow', href: '#', onclick: (e: Event) => { e.preventDefault(); go(x.page, x.label || undefined); } },
    h('span', { class: 'hp' }, t('nav.' + x.page)), x.label ? h('span', {}, x.label) : null))
    : [h('div', { class: 'none' }, t('findNone', { q: query }))]));
}

// Resize edges for the frameless window (the web view covers the whole window, so Windows never sees them).
function edges() {
  for (const dir of ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw']) {
    document.body.append(h('div', { class: 'rz ' + dir, onmousedown: (e: MouseEvent) => { if (e.button === 0) call('win', { do: 'resize', dir }); } }));
  }
  const bar = document.getElementById('titlebar')!;
  bar.addEventListener('mousedown', (e) => { if (e.button === 0 && !(e.target as HTMLElement).closest('button')) call('win', { do: 'drag' }); });
  bar.addEventListener('dblclick', (e) => { if (!(e.target as HTMLElement).closest('button')) call('win', { do: 'maximize' }); });
}

// ---- Render ---------------------------------------------------------------------------------------

/** Light or dark, then the chosen theme's colours over the tokens, so Settings looks like the top bar and the
 *  suggestion window (every theme has a light and a dark set). The default theme keeps the tokens as they are. */
const THEME_VARS: Record<string, string> = { desk: 'bg', surface: 'surface', surface2: 'surface2', line: 'line', ink: 'ink', muted: 'muted', accent: 'accent', on: 'on', sel: 'sel' };
function applyTheme() {
  const dark = effectiveDark();
  const root = document.documentElement;
  root.dataset.theme = dark ? 'dark' : 'light';
  const id = get<string>('appearance.theme');
  const th = id && id !== 'default' ? [...THEMES, ...sideThemes()].find((x) => x.id === id) : undefined;
  const set = th ? (dark ? th.dark : th.light) : {};
  for (const [k, v] of Object.entries(THEME_VARS)) {
    const c = set[k];
    if (c && /^#[0-9a-f]{3,8}$/i.test(c)) root.style.setProperty('--c-' + v, c);
    else root.style.removeProperty('--c-' + v);
  }
  // A theme's accent also gives the hover shade.
  if (set.accent) root.style.setProperty('--c-accentHi', set.accent); else root.style.removeProperty('--c-accentHi');
  // Likhon's font (Appearance): this window draws in it too, like the native windows (styles.css: --likhon-font).
  const font = get<string | null>('appearance.font');
  if (font) root.style.setProperty('--likhon-font', `"${font.replace(/["\\]/g, '')}"`); else root.style.removeProperty('--likhon-font');
}

function render() {
  applyTheme();
  const L = lang();
  document.documentElement.lang = L;
  document.title = t('window');
  renderShell();
  const main = document.getElementById('main')!;
  const keep = main.scrollTop;
  const pg = PAGES[page]();
  const sub = SUB[page] ? t(SUB[page]) : '';
  main.replaceChildren(...([
    h('h1', {}, t('nav.' + page)),
    sub ? h('p', { class: 'sub' }, sub) : null,
    h('div', { class: 'rows' }, ...pg.rows),
    pg.keys.length ? h('div', { class: 'restore' }, h('button', { type: 'button', class: 'link-btn', onclick: () => reset(pg.keys) }, t('restore'))) : null,
  ] as (HTMLElement | null)[]).filter((n): n is HTMLElement => !!n));
  main.scrollTop = keep;
}

/**
 * First run (page "agreement"): the software license agreement with Accept and Decline, in this window with no
 * navigation. Decline, or closing the window, leaves without accepting; the exe then does not start typing.
 */
async function renderAgreement() {
  applyTheme();
  const L = lang();
  document.documentElement.lang = L;
  document.title = t('agreement.title');
  document.body.classList.add('agreement');
  renderTitlebar();
  const text = await fetch(`agreement.${L}.txt`).then((r) => r.text()).catch(() => '');
  const lines = text.replace(/\r/g, '').split('\n');
  const accept = button(t('agreement.accept'), { kind: 'primary', icon: 'check', onclick: () => { call('agreementAccept'); } });
  const decline = button(t('agreement.decline'), { onclick: () => { call('agreementDecline'); } });
  document.getElementById('nav')!.replaceChildren();
  document.getElementById('main')!.replaceChildren(
    h('h1', {}, t('agreement.title')),
    h('p', { class: 'sub' }, t('agreement.intro', { date: boot.agreementDate })),
    h('pre', { class: 'agreement-text', lang: L, tabindex: '0' }, lines.join('\n')),
    h('div', { class: 'agreement-actions' }, decline, accept),
    h('p', { class: 'muted small' }, t('agreement.more', { site: host() })));
}

async function start() {
  [boot, text, icons, THEMES] = await Promise.all([
    fetch('boot.json').then((r) => r.json()),
    fetch('text.json').then((r) => r.json()),
    fetch('icons.json').then((r) => r.json()),
    fetch('themes.json').then((r) => r.json()),
  ]);
  S = boot.settings;
  if (boot.page === 'agreement') { edges(); await renderAgreement(); call('ready'); return; }
  // "tools:how" opens a tool straight away (the quick hotkeys Ctrl+Alt+H and Ctrl+Alt+S).
  const [first, sub] = boot.page.split(':');
  const asked = MOVED[first] ?? first;
  page = asked in PAGES ? asked : 'general';
  if (page === 'tools' && sub && /^[a-z]+$/.test(sub)) tool = sub;
  applyTheme();
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => { applyTheme(); render(); });
  edges();
  render();
  call('ready');
  // The top bar, the on-screen keyboard or the next-layout key may have changed a setting (the layout) while this
  // window was in the background: read the stored settings again when it comes back (boot.json is made fresh).
  addEventListener('focus', async () => {
    const fresh = await fetch('boot.json', { cache: 'no-store' }).then((r) => r.json()).catch(() => null);
    if (!fresh) return;
    S = fresh.settings;
    boot.windowsKeyboard = fresh.windowsKeyboard;
    boot.windowsKeyboardStale = fresh.windowsKeyboardStale;
    render();
  });
}

start();

export {};
