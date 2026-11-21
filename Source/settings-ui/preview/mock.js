// Browser preview of the Settings page without the exe (npm run preview): answers the page's IPC calls with
// sample data. Not part of the build; dist/ never includes it.
(() => {
  const S = {};
  const added = { layouts: [{ id: 'unijoy', kind: 'fixed', name: { en: 'UniJoy', bn: 'ইউনিজয়' } }, { id: 'khipro', kind: 'input-method', name: { en: 'Khipro' } }], themes: [] };
  // A sample chart: every letter key types a Bangla letter, Shift another.
  const chart = (id) => { const bn = 'ৃঙচছজঝঞটঠডঢণতথদধনপফবভমযরলশষসহড়ঢ়য়ৎংঃঁ'; const keys = {};
    [..."`1234567890-=qwertyuiop[]\asdfghjkl;'zxcvbnm,./"].forEach((k, i) => { keys[k] = { normal: bn[i % bn.length], shift: bn[(i + 7) % bn.length] }; });
    return { keys, name: { en: id }, author: 'Sample' }; };
  const vista = { id: 'vista', name: { en: 'Vista', bn: 'ভিস্তা' },
    light: { desk: '#E8E9E9', surface: '#FFFFFF', surface2: '#F1F2F2', line: '#C8CACA', ink: '#1A1C1D', muted: '#4E5253', accent: '#0078D4', on: '#FFFFFF', sel: '#FCEBC2' },
    dark: { desk: '#111213', surface: '#1A1C1D', surface2: '#2A2C2D', line: '#575858', ink: '#FFFFFF', muted: '#B4B5B5', accent: '#E7A810', on: '#000000', sel: '#4A3B12' } };
  const answer = {
    set: (a) => { S[a.key] = a.value; return null; },
    hotkeyFree: () => true,
    runningApps: () => [{ name: 'Notepad', exe: 'notepad.exe' }, { name: 'Microsoft Word', exe: 'WINWORD.EXE' }],
    diagPreview: () => 'Likhon diagnostics\n\n(sample)',
    diagMake: () => 'D:\Likhon\likhon-diagnostics.txt',
    sideloaded: (a) => added[a.kind] ?? [],
    layoutChart: (a) => chart(a.id),
    userList: () => [{ key: 'ইচ্ছেমতো', value: '', added: 1761100000 }],
    tool: (a) => a.name === 'taka' ? { words: 'এক হাজার দুইশো পঞ্চাশ টাকা মাত্র', amount: '৳ ১,২৫০' } : '[' + a.name + '] ' + a.text,
    bnFonts: () => ['Nirmala UI', 'Noto Sans Bengali', 'Vrinda'],
    // An earlier copy changed Windows' Bangla font: Fonts shows Restore, which puts it back.
    fontDefaultNow: () => S.fontRestored ? null : 'Li Ador Noirrit',
    fontDefault: () => { S.fontRestored = true; return { ok: true, face: null }; },
    windowsKeyboard: (a) => ({ ok: true, added: !!a.on }),
    phonetic: (a) => '[' + a.text + ']',
    checkUpdate: () => ({ ok: true, version: '0.2.0', newer: true, download: 'https://likhon.agniflux.com/en/download' }),
    libraryAdded: () => ({ 'mina': { type: 'font', version: '1.0', name: { en: 'Mina', bn: 'মিনা' } }, 'unijoy': { type: 'layout', version: '1.0', name: { en: 'UniJoy', bn: 'ইউনিজয়' } } }),
    libraryList: () => ({ ok: true, items: [
      { id: 'mina', type: 'font', name: { en: 'Mina', bn: 'মিনা' }, author: 'Suvro Kumar Das', license: 'OFL-1.1', version: '1.0', signed: false, added: '1.0', can: true },
      { id: 'dakkhinee', type: 'font', name: { en: 'Dakkhinee', bn: 'দাক্ষিণী' }, author: 'Anirban Mitra', license: 'OFL-1.1', version: '1.021', signed: false, added: null, can: true },
      { id: 'vista', type: 'theme', name: { en: 'Vista', bn: 'ভিস্তা' }, author: 'Tanbin Islam Siyam', license: 'MPL-2.0', version: '1.0', signed: false, added: null, can: true },
      { id: 'symbols-math', type: 'symbols', name: { en: 'Maths symbols', bn: 'গণিতের চিহ্ন' }, author: 'Likhon', license: 'CC0-1.0', version: '1', signed: true, added: null, can: false, page: 'https://likhon.test/en/library/symbols-math' }] }),
    libraryAdd: (a) => { if (a.item.id === 'vista') added.themes = [vista]; return { ok: true }; },
    libraryRemove: (a) => { if (a.id === 'vista') added.themes = []; return { ok: true }; },
    learnedWords: () => ['আমার', 'সোনার', 'বাংলা', 'ভালোবাসি'],
    dictDownload: () => ({ ok: true, words: 159380, version: 'base' }),
    donate: (a) => ({ ok: true, accepted: a.words.length }),
  };
  window.ipc = {
    postMessage(m) {
      const { id, cmd, args } = JSON.parse(m);
      console.log('ipc', cmd, JSON.stringify(args));
      setTimeout(() => window.__likhon.reply(id, answer[cmd] ? answer[cmd](args) : null), 0);
    },
  };
})();
