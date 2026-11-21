// Converts Avro Keyboard's word database (Database.db3: one table of Bangla words per first letter, plus a Suffix
// table) and its phonetic autocorrect list (autodict.dct, a text file) into Likhon's dictionary file,
// assets/dict/mpl/words.sqlite. Build-time only. Every built-in list is SQLite (owner decision 2026-10-22).
//
//   node scripts/avro/convert-dictionary.mjs <Database.db3> <autodict.dct> <commit>
//
// Take both from github.com/mugli/Avro-Keyboard (assets/) at a pinned commit. They are MPL-2.0 (owner decision
// 2026-10-22), so the converted file stays MPL-2.0 and sits beside LICENSE and SOURCE.md. The file itself does not
// name its source (owner decision 2026-10-22: no dictionary references to Avro); SOURCE.md and the About credits do.
//
// Likhon dictionary (format likhon-dictionary, version 1), read-only at run time:
//   meta(key, value)            format, version, license, words, suffixes, autocorrect
//   words(word)                 one Bangla word per row, NFC, no duplicates, sorted (WITHOUT ROWID: a prefix search
//                               is one index range scan)
//   suffixes(roman, bangla)     endings typed after a word, e.g. "er" → "ের"
//   autocorrect(roman, phonetic) a word typed in Latin letters and the phonetic spelling it is typed as instead,
//                               e.g. "account" → "oZakaunT" (অ্যাকাউন্ট); the app turns the spelling into Bangla
import fs from 'node:fs';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { fileURLToPath } from 'node:url';

const [src, autodict, commit] = process.argv.slice(2);
if (!src || !autodict || !/^[0-9a-f]{40}$/.test(commit ?? '')) { console.error('usage: convert-dictionary.mjs <Database.db3> <autodict.dct> <40-character commit>'); process.exit(1); }
const outDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../assets/dict/mpl');
fs.mkdirSync(outDir, { recursive: true });
const outFile = path.join(outDir, 'words.sqlite');
fs.rmSync(outFile, { force: true });

const avro = new DatabaseSync(src, { readOnly: true });
const tables = avro.prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name <> 'Suffix'").all().map((r) => r.name);
const BANGLA = /^[\u0980-\u09FF\u200C\u200D]+$/;
const words = new Set();
let skipped = 0;
for (const t of tables) {
  for (const { Words } of avro.prepare(`SELECT Words FROM "${t.replace(/"/g, '')}"`).all()) {
    const w = String(Words ?? '').trim().normalize('NFC');
    if (BANGLA.test(w)) words.add(w); else if (w) skipped++;
  }
}
const suffixes = new Map();
for (const { English, Bangla } of avro.prepare('SELECT English, Bangla FROM Suffix').all()) {
  const r = String(English ?? '').trim(), b = String(Bangla ?? '').trim().normalize('NFC');
  if (/^[A-Za-z`]+$/.test(r) && BANGLA.test(b)) suffixes.set(r, b);
}
avro.close();

// autodict.dct: "/" comment lines, then "typed spelling" per line. Only words are kept (letters, digits, ` and -,
// at least one letter); emoticon rows and rows that change nothing are left out.
const WORD = /^(?=.*[A-Za-z])[A-Za-z0-9`-]+$/;
const autocorrect = new Map();
let autoSkipped = 0;
for (const line of fs.readFileSync(autodict, 'latin1').split(/\r?\n/)) {
  if (!line.trim() || line.startsWith('/')) continue;
  const [r, p, ...more] = line.trim().split(/\s+/);
  if (WORD.test(r) && WORD.test(p ?? '') && !more.length && r !== p) autocorrect.set(r, p); else autoSkipped++;
}

const db = new DatabaseSync(outFile);
db.exec(`PRAGMA page_size = 4096; PRAGMA journal_mode = OFF;
  CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
  CREATE TABLE words (word TEXT PRIMARY KEY) WITHOUT ROWID;
  CREATE TABLE suffixes (roman TEXT PRIMARY KEY, bangla TEXT NOT NULL) WITHOUT ROWID;
  CREATE TABLE autocorrect (roman TEXT PRIMARY KEY, phonetic TEXT NOT NULL) WITHOUT ROWID;`);
db.exec('BEGIN');
const addWord = db.prepare('INSERT INTO words (word) VALUES (?)');
for (const w of [...words].sort()) addWord.run(w);
const addSuffix = db.prepare('INSERT INTO suffixes (roman, bangla) VALUES (?, ?)');
for (const [r, b] of [...suffixes].sort()) addSuffix.run(r, b);
const addAuto = db.prepare('INSERT INTO autocorrect (roman, phonetic) VALUES (?, ?)');
for (const [r, p] of [...autocorrect].sort()) addAuto.run(r, p);
const addMeta = db.prepare('INSERT INTO meta (key, value) VALUES (?, ?)');
for (const [k, v] of Object.entries({
  format: 'likhon-dictionary', version: '1', license: 'MPL-2.0', words: String(words.size), suffixes: String(suffixes.size),
  autocorrect: String(autocorrect.size),
})) addMeta.run(k, v);
db.exec('COMMIT');
db.exec('VACUUM');
db.close();

fs.writeFileSync(path.join(outDir, 'SOURCE.md'), `# Dictionary converted from Avro Keyboard

\`words.sqlite\` is converted from Avro Keyboard's word database and phonetic autocorrect list by
\`scripts/avro/convert-dictionary.mjs\`. Avro Keyboard is
Copyright OmicronLab and is published under the Mozilla Public License 2.0 (\`LICENSE\` in this folder), so this file is
covered by the same licence and stays in this folder.

Sources:
- https://github.com/mugli/Avro-Keyboard/blob/${commit}/assets/Database.db3
- https://github.com/mugli/Avro-Keyboard/blob/${commit}/assets/autodict.dct

Changes from the source: the 48 per-letter tables are merged into one sorted \`words\` table (Unicode NFC, duplicates and
non-Bangla rows removed), the Suffix table becomes \`suffixes\`, the autocorrect text file becomes the \`autocorrect\` table
(word rows only; emoticon rows and rows that change nothing left out), and a \`meta\` table names the format and licence.
Words: ${words.size}. Suffixes: ${suffixes.size}. Autocorrect: ${autocorrect.size}. Rows left out: ${skipped} words,
${autoSkipped} autocorrect.
`);
console.log(`words.sqlite: ${words.size} words, ${suffixes.size} suffixes, ${autocorrect.size} autocorrect, ${skipped}+${autoSkipped} rows left out, ${(fs.statSync(outFile).size / 1048576).toFixed(2)} MB`);
