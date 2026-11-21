// Converts Avro Keyboard's .avrolayout files (XML with base64 preview bitmaps) into Likhon layout files (JSON,
// shared/schemas/layout.schema.json). Build-time only; the output is committed under assets/layouts/avro/.
//
//   node scripts/avro/convert-layouts.mjs <folder with the .avrolayout files> <commit>
//
// Take the inputs from github.com/mugli/Avro-Keyboard (assets/keyboard-layouts) at a pinned commit, never from an
// installed copy. The files are MPL-2.0 (owner decision 2026-10-22), so the JSON stays MPL-2.0: it is written beside
// LICENSE and SOURCE.md, and each file names its source. The preview bitmaps are dropped (the layout viewer draws
// the keys itself).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const [src, commit] = process.argv.slice(2);
if (!src || !/^[0-9a-f]{40}$/.test(commit ?? '')) { console.error('usage: convert-layouts.mjs <folder> <40-character commit>'); process.exit(1); }
const out = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../assets/layouts/avro');
fs.mkdirSync(out, { recursive: true });

// Avro key names (Windows virtual-key names) to the character the key types on a US keyboard, unshifted.
const KEYS = { OEM3: '`', MINUS: '-', PLUS: '=', OEM4: '[', OEM6: ']', OEM5: '\\', OEM1: ';', OEM7: "'", COMMA: ',', PERIOD: '.', OEM2: '/' };
const keyName = (k) => KEYS[k] ?? k.toLowerCase();
const LEVELS = { Normal: 'normal', Shift: 'shift', AltGr: 'altgr', ShiftAltGr: 'shiftAltgr' };
const NUMPAD = { NumAdd: '+', NumSubtract: '-', NumMultiply: '*', NumDivide: '/', NumDecimal: '.' };
// Likhon ids and names (short names: Jatiya/জাতীয়, Probhat/প্রভাত).
const NAMES = {
  'Avro Easy': ['avro-easy', 'Avro Easy', 'অভ্র ইজি'],
  Bornona: ['bornona', 'Bornona', 'বর্ণনা'],
  'Munir Optima (uni)': ['munir', 'Munir Optima', 'মুনীর অপটিমা'],
  'National (Jatiya)': ['jatiya', 'Jatiya', 'জাতীয়'],
  Probhat: ['probhat', 'Probhat', 'প্রভাত'],
};

const cdata = (xml, tag) => {
  const m = xml.match(new RegExp(`<${tag}>\\s*<!\\[CDATA\\[([\\s\\S]*?)\\]\\]>\\s*</${tag}>`));
  return m ? m[1] : null;
};
const unescape = (s) => s.replace(/&apos;/g, "'").replace(/&quot;/g, '"').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');

const sources = [];
for (const file of fs.readdirSync(src).filter((f) => f.endsWith('.avrolayout')).sort()) {
  const xml = fs.readFileSync(path.join(src, file), 'utf8').replace(/^\uFEFF/, '');
  const avroName = cdata(xml, 'LayoutName').trim();
  const [id, en, bn] = NAMES[avroName] ?? (() => { throw new Error(`no Likhon name for "${avroName}" (${file})`); })();
  const keys = {};
  for (const m of xml.matchAll(/<Key_([A-Z0-9]+)_(Normal|Shift|AltGr|ShiftAltGr)>\s*<!\[CDATA\[([\s\S]*?)\]\]>/g)) {
    const [, k, level, value] = m;
    if (!value) continue;
    // Avro writes ' " < > & as XML entities inside CDATA; the key types the character itself.
    (keys[keyName(k)] ??= {})[LEVELS[level]] = unescape(value);
  }
  const numpad = {};
  for (const m of xml.matchAll(/<(Num[0-9]|NumAdd|NumSubtract|NumMultiply|NumDivide|NumDecimal)>\s*<!\[CDATA\[([\s\S]*?)\]\]>/g)) {
    if (m[2]) numpad[NUMPAD[m[1]] ?? m[1].slice(3)] = unescape(m[2]);
  }
  const layout = {
    format: 'likhon-layout',
    version: 1,
    id,
    name: { en, bn },
    kind: 'fixed',
    author: cdata(xml, 'DeveloperName').trim(),
    description: unescape(cdata(xml, 'DeveloperComment') ?? '').trim().replace(/\r\n/g, '\n').replace(/^N\/A$/, ''),
    layoutVersion: cdata(xml, 'LayoutVersion').trim(),
    license: 'MPL-2.0',
    source: `https://github.com/mugli/Avro-Keyboard/blob/${commit}/assets/keyboard-layouts/${encodeURIComponent(file)}`,
    keys,
    numpad,
  };
  fs.writeFileSync(path.join(out, id + '.json'), JSON.stringify(layout, null, 2) + '\n');
  sources.push(`| \`${id}.json\` | ${file} | ${layout.author} |`);
  console.log(`${id}.json: ${Object.keys(keys).length} keys, ${Object.keys(numpad).length} numpad`);
}

fs.writeFileSync(path.join(out, 'SOURCE.md'), `# Layouts converted from Avro Keyboard

These files are converted from Avro Keyboard's layout files by \`scripts/avro/convert-layouts.mjs\`. Avro Keyboard is
Copyright OmicronLab and is published under the Mozilla Public License 2.0 (\`LICENSE\` in this folder). These converted
files are covered by the same licence, so they stay in this folder, unmixed with Likhon's own files.

Source: https://github.com/mugli/Avro-Keyboard/tree/${commit}/assets/keyboard-layouts

| Likhon file | Avro file | Layout author (as the Avro file names them) |
| --- | --- | --- |
${sources.join('\n')}

Changes from the source: the format is JSON instead of XML, the preview bitmaps are left out, and each layout has a
Likhon id and an English and Bangla name. The key assignments are unchanged.
`);
