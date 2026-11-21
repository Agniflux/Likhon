// Writes assets/licenses/rust-crates.txt (not to be edited by hand): the license notices of every Rust crate compiled into what Likhon ships
// (likhon.exe, likhon_tsf.dll, and the website's typing engine, crates/likhon-wasm). See LICENSING.md at the root.
//
//   node scripts/licenses/build-notices.mjs          (from Desktop/Source; run again after any Cargo.lock change)
//   node scripts/licenses/build-notices.mjs --check  (exit 1 if rust-crates.txt is out of date)
//
// How it works: `cargo metadata` gives the resolved dependency graph for the Windows (x86_64) and WebAssembly
// targets. From likhon, likhon-tsf and likhon-wasm it follows normal dependencies only: build scripts, dev
// dependencies and proc-macro crates run on the build machine and are not part of the shipped files, so they are
// left out. For each crate it copies the licence files the crate itself publishes (LICENSE*, COPYING*, NOTICE*).
// Where a crate offers a choice ("MIT OR Apache-2.0"), Likhon takes MIT when offered (shortest notice), else a
// license that asks for no notice, else the first choice. Identical licence texts are printed once, with every crate that uses them listed above the text.
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const source = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const out = path.join(source, 'assets/licenses/rust-crates.txt');
const check = process.argv.includes('--check');

const metadata = (dir, target) => JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--filter-platform', target], { cwd: dir, maxBuffer: 256 * 1024 * 1024, encoding: 'utf8' }));

/** Packages reachable from the roots through normal (shipped) dependencies, proc-macros excluded. */
function shipped(meta, roots) {
  const pkgs = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const isMacro = (p) => p.targets.some((t) => t.kind.includes('proc-macro'));
  const seen = new Set();
  const stack = meta.packages.filter((p) => roots.includes(p.name) && !p.source).map((p) => p.id);
  if (stack.length !== roots.length) throw new Error(`roots not found: ${roots.join(', ')}`);
  while (stack.length) {
    const id = stack.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const d of nodes.get(id).deps) {
      if (!d.dep_kinds.some((k) => k.kind === null)) continue; // normal dependencies only
      const p = pkgs.get(d.pkg);
      if (!isMacro(p)) stack.push(d.pkg);
    }
  }
  return [...seen].map((id) => pkgs.get(id)).filter((p) => p.source); // registry crates only, not Likhon's own
}

const exe = shipped(metadata(source, 'x86_64-pc-windows-msvc'), ['likhon', 'likhon-tsf']);
const wasm = shipped(metadata(path.join(source, 'crates/likhon-wasm'), 'wasm32-unknown-unknown'), ['likhon-wasm']);
const all = new Map();
for (const [list, where] of [[exe, 'app'], [wasm, 'web']]) {
  for (const p of list) {
    const key = `${p.name} ${p.version}`;
    const e = all.get(key) ?? { p, where: new Set() };
    e.where.add(where);
    all.set(key, e);
  }
}

// Licenses that ask for no notice in copies: the crate is listed, but no text is printed for it.
const NO_NOTICE = ['CC0-1.0', 'MIT-0', '0BSD', 'Unlicense'];

/** The license Likhon takes for a crate, from its SPDX expression: MIT where offered, else a no-notice license,
 *  else the first choice. In "(A OR B) AND C" the choice is made inside the brackets and C is kept. */
function choose(expr) {
  const pickOne = (alts) => {
    const opts = alts.split(/\s+OR\s+/).map((x) => x.trim()).filter(Boolean);
    return opts.includes('MIT') ? 'MIT' : opts.find((o) => NO_NOTICE.includes(o)) ?? opts[0];
  };
  let e = (expr || '').replace(/\//g, ' OR ').trim();
  if (!e) return null;
  e = e.replace(/\(([^()]+)\)/g, (_, inner) => pickOne(inner));
  return /\bAND\b/.test(e) ? e.split(/\s+AND\s+/).map(pickOne).join(' AND ') : pickOne(e);
}

/** The license files a crate publishes for the chosen license (all of them when names do not tell). */
function licenseFiles(p, chosen) {
  if (NO_NOTICE.includes(chosen)) return [];
  const dir = path.dirname(p.manifest_path);
  const files = fs.readdirSync(dir).filter((f) => /^(LICEN[CS]E|COPYING|NOTICE|UNLICENSE)/i.test(f) && !/\.spdx$/i.test(f) && fs.statSync(path.join(dir, f)).isFile()).sort();
  const plain = files.filter((g) => !/^NOTICE/i.test(g));
  const tag = { MIT: /MIT/, 'Apache-2.0': /APACHE/, 'Unicode-3.0': /UNICODE/, Zlib: /ZLIB/ };
  const parts = chosen.split(' AND ');
  const pick = files.filter((f) => {
    if (/^NOTICE/i.test(f)) return true; // Apache-2.0 section 4(d): NOTICE files always travel with the code
    const u = f.toUpperCase();
    if (plain.length === 1) return true;
    return parts.some((part) => (tag[part] ? tag[part].test(u) : true));
  });
  const found = (pick.length ? pick : files).map((f) => path.join(dir, f));
  // A few crates publish no license file: their text is kept in scripts/licenses/overrides/<name>.txt (see its README).
  const override = path.join(source, 'scripts/licenses/overrides', `${p.name}.txt`);
  if (!found.length && fs.existsSync(override)) found.push(override);
  return found.map((f) => ({ f, text: fs.readFileSync(f, 'utf8').replace(/\r\n/g, '\n').trim() }));
}

const groups = new Map(); // license text, whitespace-insensitive -> { text, crates }
const table = [];
const missing = [];
for (const { p, where } of [...all.values()].sort((a, b) => a.p.name.localeCompare(b.p.name) || a.p.version.localeCompare(b.p.version))) {
  const chosen = choose(p.license);
  const files = licenseFiles(p, chosen);
  if (!files.length && !NO_NOTICE.includes(chosen)) missing.push(`${p.name} ${p.version} (${p.license || p.license_file || 'no license field'})`);
  table.push([`${p.name} ${p.version}`, chosen ?? '?', [...where].sort().join('+'), (p.repository ?? '').replace(/\/$/, '')]);
  for (const { text } of files) {
    const key = text.replace(/\s+/g, ' ');
    const g = groups.get(key) ?? { text, crates: [] };
    g.crates.push(`${p.name} ${p.version}`);
    groups.set(key, g);
  }
}
if (missing.length) {
  console.error('No license file found for:\n  ' + missing.join('\n  ') + '\nAdd the text to scripts/licenses/overrides/ before shipping.');
  process.exit(1);
}

const head = ['Library', 'License used', 'In', 'Source'];
const widths = head.slice(0, 3).map((h, i) => Math.max(h.length, ...table.map((r) => r[i].length)));
const line = (r) => r.map((c, i) => (i < 3 ? c.padEnd(widths[i]) : c)).join('  ').trimEnd();
const noNotice = table.filter((r) => NO_NOTICE.includes(r[1])).map((r) => r[0]);

const text = `These open-source Rust libraries are compiled into likhon.exe and likhon_tsf.dll ("app") and into the
typing engine the Likhon website runs in your browser ("web"). Where a library offers a choice of licenses, Likhon
uses it under the license named here.

${line(head)}
${table.map(line).join('\n')}

libsqlite3-sys includes SQLite (https://sqlite.org), which its authors have dedicated to the public domain.${noNotice.length ? `
${noNotice.join(', ')}: used under a license that asks for no notice, so no text is printed below.` : ''}

The license texts follow. Each is printed once, under the libraries it applies to.
${[...groups.values()].map(({ text: t, crates }) => `\n---- ${crates.join(', ')} ----\n\n${t}\n`).join('')}`;

if (check) {
  const old = fs.existsSync(out) ? fs.readFileSync(out, 'utf8') : '';
  if (old !== text) { console.error('assets/licenses/rust-crates.txt is out of date: run node scripts/licenses/build-notices.mjs'); process.exit(1); }
  console.log('rust-crates.txt is up to date');
} else {
  fs.writeFileSync(out, text);
  console.log(`rust-crates.txt: ${all.size} crates (${exe.length} app, ${wasm.length} web), ${groups.size} license texts, ${(text.length / 1024).toFixed(0)} KB`);
}
