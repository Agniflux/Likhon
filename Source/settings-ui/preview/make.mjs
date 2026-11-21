// Copies dist/ into preview/site/ with the IPC mock and a sample boot.json, for checking the page in a browser.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const site = path.join(here, 'site');
fs.rmSync(site, { recursive: true, force: true });
fs.cpSync(path.join(here, '..', 'dist'), site, { recursive: true });
fs.copyFileSync(path.join(here, 'mock.js'), path.join(site, 'mock.js'));
const index = path.join(site, 'index.html');
fs.writeFileSync(index, fs.readFileSync(index, 'utf8').replace('<script type="module"', '<script src="mock.js"></script>\n  <script type="module"'));
const lang = process.argv[2] === 'bn' ? 'bn' : 'en';
const page = process.argv[3] || 'general';
fs.writeFileSync(path.join(site, 'boot.json'), JSON.stringify({
  settings: { language: lang }, page, version: '0.1.0', edition: 'portable', site: 'https://likhon.test', startWithWindows: false, learned: 4, windowsKeyboard: false, windows: 'Windows 11 24H2 (build 26100)',
  dataDir: 'D:\Likhon', dataPortable: true, diagnostics: null, dictionary: { words: 159380, path: 'D:\Likhon\dict\words.sqlite', license: 'MPL-2.0' },
  defaults: { language: 'en', color: 'system', 'topbar.mode': 'always', start_with_windows: false, 'typing.suggestions': true, 'typing.suggestion_count': 5,
    'typing.next_word': true, 'typing.next_count': 3, 'typing.autocorrect': true, 'typing.emoji': false, 'typing.bangla_digits': false, 'typing.dari': true,
    'typing.learn': true, layout: 'phonetic', output: 'unicode', 'hotkeys.toggle': { vk: 123, ctrl: false, alt: false, shift: false, label: 'F12' },
    'apprules.passthrough': [{ name: 'Google Chrome', exe: 'chrome.exe' }, { name: 'Mozilla Firefox', exe: 'firefox.exe' }, { name: 'Visual Studio Code', exe: 'Code.exe' }],
    'appearance.theme': null, 'appearance.font_size': 17, 'appearance.row_height': 30 },
}));
console.log('preview/site ready:', lang, page);
