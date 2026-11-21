// Adds or replaces phrases in one phrases file, from a JSON map { "id": ["English", "Bangla"] }.
// Bangla is marked "needs review" and bnFrom is set from the English text. Use null to delete an id.
//
//   node set-phrases.mjs phrases/web/header.json changes.json
//   node set-phrases.mjs phrases/web/header.json --prune web.header.nav.   (deletes every id with that prefix first)
import fs from 'node:fs';
import crypto from 'node:crypto';

const [file, ...rest] = process.argv.slice(2);
if (!file) { console.error('usage: set-phrases.mjs <phrases file> [--prune <prefix>]... [changes.json]'); process.exit(1); }
const data = fs.existsSync(file) ? JSON.parse(fs.readFileSync(file, 'utf8')) : {};
const sha = (s) => crypto.createHash('sha1').update(s, 'utf8').digest('hex').slice(0, 8);

let changes = {};
for (let i = 0; i < rest.length; i++) {
  if (rest[i] === '--prune') { const p = rest[++i]; for (const k of Object.keys(data)) if (k.startsWith(p)) delete data[k]; }
  else changes = { ...changes, ...JSON.parse(fs.readFileSync(rest[i], 'utf8')) };
}
for (const [id, v] of Object.entries(changes)) {
  if (v === null) { delete data[id]; continue; }
  const [en, bn] = v;
  data[id] = { en, bn: bn ?? '', status: bn ? 'needs review' : 'draft', ...(bn ? { bnFrom: sha(en) } : {}) };
}
fs.writeFileSync(file, JSON.stringify(data, null, 2) + '\n');
console.log(`${file}: ${Object.keys(changes).length} changes, ${Object.keys(data).length} phrases`);
