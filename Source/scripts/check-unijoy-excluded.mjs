// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// Fails (exit 1) if a UniJoy layout file is anywhere in what a release or the library would carry. UniJoy's file names
// no licence, so it never goes into a release bundle, the public repository or the library (Software licensing terms,
// License notices). Run it in CI before packaging and again on the outputs:
//
//   node Desktop/Source/scripts/check-unijoy-excluded.mjs <folder or file>...      (no argument: the source tree)
//
// A path counts when its name contains "unijoy" or a .json layout file names itself "unijoy" (id or name). Prose
// that merely mentions UniJoy (the licence notes, the engine's tests that skip when the file is absent) is fine.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const roots = process.argv.length > 2 ? process.argv.slice(2) : [path.join(here, '..'), path.join(here, '..', '..', 'Linux'), path.join(here, '..', '..', 'Mac')];
const SKIP = new Set(['node_modules', 'target', '.git', 'dist', '__pycache__']);
const found = [];

function scan(p) {
  let st;
  try { st = fs.statSync(p); } catch { return; }
  if (st.isDirectory()) {
    if (SKIP.has(path.basename(p)) || path.basename(p).startsWith('target')) return;
    for (const f of fs.readdirSync(p)) scan(path.join(p, f));
    return;
  }
  if (path.basename(p) === 'check-unijoy-excluded.mjs' || /\.(md|txt)$/i.test(p)) return; // prose (the licence note) is fine
  if (/unijoy/i.test(path.basename(p)) || /unijoy/i.test(path.basename(path.dirname(p)))) { found.push(p); return; }
  if (p.endsWith('.json') && st.size < 5_000_000) {
    try {
      const j = JSON.parse(fs.readFileSync(p, 'utf8'));
      const names = [j.id, j.name, j.title, j.layout].filter((x) => typeof x === 'string');
      if (names.some((x) => /^unijoy$/i.test(x.trim()))) found.push(p);
    } catch { /* not JSON we care about */ }
  }
}

for (const r of roots) scan(r);
if (found.length) {
  console.error('UniJoy must not ship. Found:\n  ' + found.join('\n  '));
  process.exit(1);
}
console.log('check-unijoy-excluded: nothing found in ' + roots.length + ' location(s)');
