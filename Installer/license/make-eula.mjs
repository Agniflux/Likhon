// Makes the installer's licence pages (eula.en.txt, eula.bn.txt) from the website's agreement, so the installer shows
// the same text as https://likhon.agniflux.com/en/legal/license (the site's own words: "The installer shows the same text").
//
//   node Desktop/Installer/license/make-eula.mjs
//
// Source: Web/Laravel/resources/legal/{en,bn}/license.md. Dropped: the front matter and the [!LAWYER] review notes
// (they are for the owner and the lawyer, not for the person installing). Links keep their text only. The output is
// plain text, CRLF line ends, UTF-8 with a byte-order mark (Inno Setup shows Bangla correctly from that).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const legal = path.join(here, '..', '..', '..', 'Web', 'Laravel', 'resources', 'legal');

const titles = {
  en: (v) => `Likhon software license agreement\nVersion ${v}`,
  bn: (v) => `লিখন সফটওয়্যার লাইসেন্স চুক্তি\nসংস্করণ ${v}`,
};

function plain(md, lang, version) {
  const m = md.replace(/^﻿/, '').match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]*)$/);
  if (!m) throw new Error('no front matter');
  const updated = (m[1].match(/^updated:\s*(\S+)/m) || [])[1] || '';
  const out = [];
  let skipping = false;
  for (const raw of m[2].split(/\r?\n/)) {
    // A [!LAWYER] block is every following line that starts with ">".
    if (/^>\s*\[!LAWYER\]/.test(raw)) { skipping = true; continue; }
    if (skipping && /^>/.test(raw)) continue;
    skipping = false;
    let line = raw
      .replace(/^#{1,6}\s+/, '')               // headings: keep the words
      .replace(/\s*\{#[a-z-]+\}\s*$/, '')       // heading anchors
      .replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')  // links: keep the text
      .replace(/<([^>@\s]+@[^>\s]+)>/g, '$1')   // <mail@example.com>
      .replace(/\*\*([^*]+)\*\*/g, '$1')        // bold
      .replace(/`([^`]+)`/g, '$1');             // code
    out.push(line);
  }
  let body = out.join('\n').replace(/\n{3,}/g, '\n\n').trim();
  const head = titles[lang](version) + (updated ? (lang === 'bn' ? `\nশেষ হালনাগাদ: ${updated}` : `\nLast updated: ${updated}`) : '');
  return `${head}\n\n${body}\n`;
}

const cargo = fs.readFileSync(path.join(here, '..', '..', 'Source', 'Cargo.toml'), 'utf8');
const version = cargo.match(/^version\s*=\s*"([^"]+)"/m)[1];
for (const lang of ['en', 'bn']) {
  const md = fs.readFileSync(path.join(legal, lang, 'license.md'), 'utf8');
  const text = plain(md, lang, version).replace(/\n/g, '\r\n');
  const file = path.join(here, `eula.${lang}.txt`);
  fs.writeFileSync(file, '﻿' + text, 'utf8');
  console.log(`${path.basename(file)}: ${text.length} characters`);
}
