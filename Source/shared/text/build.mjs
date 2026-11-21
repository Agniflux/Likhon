#!/usr/bin/env node
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Likhon text library build.
// Reads terms.json and phrases/**/*.json, checks them, resolves term references,
// and writes one bundle per language plus a report of missing or stale Bangla.
//
//   node build.mjs --out <dir> [--prefix web.] [--mode normal|ids|long] [--check]
//
// --prefix  only include phrases whose ID starts with this (repeatable)
// --mode    ids: show IDs instead of text (debug); long: stretch every string (layout test)
// --check   exit with an error if any Bangla is missing or stale (for CI later)

import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (name, def) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : def; };
const optAll = (name) => args.flatMap((a, i) => (a === name ? [args[i + 1]] : []));
const outDir = opt('--out');
const mode = opt('--mode', process.env.LIKHON_TEXT_MODE || 'normal');
const prefixes = optAll('--prefix');
const check = args.includes('--check');

const STATUSES = ['draft', 'needs review', 'reviewed'];
export const hash = (s) => crypto.createHash('sha1').update(s).digest('hex').slice(0, 8);

const errors = [];
const fail = (msg) => errors.push(msg);

// ---- load ---------------------------------------------------------------
const terms = JSON.parse(fs.readFileSync(path.join(here, 'terms.json'), 'utf8'));
const phrases = {};
const phraseFile = {};
const walk = (dir) => {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p);
    else if (e.name.endsWith('.json')) {
      const data = JSON.parse(fs.readFileSync(p, 'utf8'));
      for (const [id, v] of Object.entries(data)) {
        if (phrases[id]) fail(`duplicate phrase ID ${id} in ${path.relative(here, p)} and ${phraseFile[id]}`);
        phrases[id] = v;
        phraseFile[id] = path.relative(here, p);
      }
    }
  }
};
walk(path.join(here, 'phrases'));

// ---- check --------------------------------------------------------------
for (const [id, t] of Object.entries(terms)) {
  if (!id.startsWith('term.')) fail(`term ID must start with "term.": ${id}`);
  if (typeof t.en !== 'string' || !t.en) fail(`${id}: missing English`);
  if (!STATUSES.includes(t.status)) fail(`${id}: unknown status "${t.status}"`);
}
const REF = /\{(term\.[a-z0-9_]+)(?::([a-z0-9_]+))?\}/g;
for (const [id, p] of Object.entries(phrases)) {
  if (typeof p.en !== 'string' || !p.en) fail(`${id}: missing English`);
  if (typeof p.bn !== 'string') fail(`${id}: bn must be a string (empty if not written yet)`);
  if (!STATUSES.includes(p.status)) fail(`${id}: unknown status "${p.status}"`);
  for (const [, ref] of `${p.en} ${p.bn}`.matchAll(REF)) if (!terms[ref]) fail(`${id}: unknown term ${ref}`);
}
if (errors.length) {
  console.error('Text library has errors:\n  ' + errors.join('\n  '));
  process.exit(1);
}

// ---- resolve ------------------------------------------------------------
// A Bangla phrase is usable only if it is written, is not stale, and every term it uses has Bangla.
const missing = [];   // no Bangla at all
const stale = [];     // English changed after the Bangla was written
const termGaps = [];  // Bangla uses a term (or form) that has no Bangla yet
const unreviewed = []; // usable, but not reviewed

const enOf = (s) => s.replace(REF, (_, ref) => terms[ref].en);
const bnOf = (id, s) => {
  let ok = true;
  const out = s.replace(REF, (_, ref, form) => {
    const b = terms[ref].bn || {};
    const v = form ? b.forms?.[form] : b.plain;
    if (!v) { ok = false; termGaps.push(`${id} uses ${ref}${form ? ':' + form : ''}`); }
    return v || '';
  });
  return ok ? out : null;
};

const bundles = { en: {}, bn: {} };
const fallback = [];
const included = Object.keys(phrases).filter((id) => !prefixes.length || prefixes.some((p) => id.startsWith(p))).sort();
for (const id of included) {
  const p = phrases[id];
  const en = enOf(p.en);
  let bn = null;
  if (!p.bn) missing.push(id);
  else if (p.bnFrom !== hash(p.en)) stale.push(id);
  else bn = bnOf(id, p.bn);
  if (bn !== null && p.status !== 'reviewed') unreviewed.push(id);
  if (bn === null) fallback.push(id);
  bundles.en[id] = en;
  bundles.bn[id] = bn ?? en;
}

// Debug modes from the language brief.
const stretch = (s) => s + ' ' + s.slice(0, Math.ceil(s.length * 0.4));
if (mode === 'ids') for (const l of ['en', 'bn']) for (const id of included) bundles[l][id] = id;
if (mode === 'long') for (const l of ['en', 'bn']) for (const id of included) bundles[l][id] = stretch(bundles[l][id]);

// ---- write --------------------------------------------------------------
const report = [
  '# Text library report',
  '',
  `Phrases: ${included.length}. Bangla shown: ${included.length - fallback.length}. English fallback: ${fallback.length}. Not yet reviewed: ${unreviewed.length}.`,
  '',
  `## Missing Bangla (${missing.length})`, ...missing.map((x) => `- ${x}`), '',
  `## Stale Bangla: English changed since it was written (${stale.length})`, ...stale.map((x) => `- ${x}`), '',
  `## Bangla waits on a term (${termGaps.length})`, ...termGaps.map((x) => `- ${x}`), '',
].join('\n');

if (outDir) {
  fs.mkdirSync(outDir, { recursive: true });
  fs.writeFileSync(path.join(outDir, 'en.json'), JSON.stringify(bundles.en, null, 1));
  fs.writeFileSync(path.join(outDir, 'bn.json'), JSON.stringify(bundles.bn, null, 1));
  fs.writeFileSync(path.join(outDir, 'fallback.json'), JSON.stringify(fallback));
  fs.writeFileSync(path.join(outDir, 'report.md'), report);
}
console.log(`text: ${included.length} phrases, ${fallback.length} English fallback, ${stale.length} stale, ${unreviewed.length} Bangla not reviewed${mode !== 'normal' ? `, mode ${mode}` : ''}`);
if (check && (missing.length || stale.length || termGaps.length)) process.exit(2);
