// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Exports icons from the duotone pack (Reference/design-v2/icons.js) at build time. The pack's generator
// never ships; consumers embed only the element lists for the names they use.
//
//   import { iconSet } from '<repo>/Desktop/Source/shared/icons/build-icons.mjs';
//   const icons = iconSet(['download', 'lock']);   // { name: [[tag, attrs], ...] }; throws on an unknown name
//
//   node Desktop/Source/shared/icons/build-icons.mjs [outDir]   // every icon as an .svg with the default
//                                                               // colours (for export and review), plus names.json
//
// Colours in the element lists: the primary colour is "currentColor"; the secondary colour becomes
// class "i2" (fill) or "i2s" (stroke). Consumers style them from the tokens, for example
// `.icon { color: var(--c-accent) } .icon .i2 { fill: var(--c-sel) } .icon .i2s { stroke: var(--c-sel) }`.
// No inline styles, so the strict content security policy on the website still holds.
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
export const PACK = path.resolve(here, '../../../Reference/design-v2/icons.js');

// Names other code used before v2 (Lucide) or that the designs reach for, mapped to pack names.
// Keep this short: prefer the pack name in new code. Entries marked provisional wait for design sign-off.
export const ALIASES = {
  'triangle-alert': 'alert-triangle',
  'circle-x': 'x-circle',
  'clock-3': 'clock',
  'circle-arrow-down': 'arrow-down-circle',
  file: 'file-text',
  signature: 'key', // Admin prototype uses "key" for Ready to sign
  package: 'download', // Admin prototype uses "download" for App releases
  inbox: 'list-checks', // Admin prototype uses "list-checks" for the queue
  image: 'file-text', // provisional: "image" was removed from the pack on 2026-10-21; Admin (Site artwork) and Desktop still ask for it
  'share-2': 'cloud-upload', // as the Desktop prototype maps it
  'bangladeshi-taka': 'taka-sign',
  'circle-question-mark': 'help-circle',
};

/** The Likhon quill (the brand mark, Reference/design-v2/Likhon Desktop.dc.html §1), filled, on the mark's 1200-unit
 * grid; the design crops it to the box 150..1050. */
export const QUILL = 'M961.5,200c-111.9,7-229.7,44-319.6,96c-14,8-25,19-32,33l-34,72v-72c-123.8,69-240.7,179-285.6,299c-24,64-18,128,10,176c89.9-136,239.7-299,410.5-407c-150.8,123-310.6,323-480.4,603l74.9-29c23-43,46.9-84,77.9-125c176.8,4,336.6-146,412.5-269L606,621c93.9-29,173.8-61,219.7-113c53.9-60,113.9-178,143.8-296C972,202,969.5,199.5,961.5,200z';

const SECONDARY = '#ABCDEF'; // sentinel, replaced by a class

let pack;
function load() {
  if (pack) return pack;
  const ctx = {};
  vm.createContext(ctx);
  vm.runInContext(fs.readFileSync(PACK, 'utf8'), ctx, { filename: PACK });
  pack = ctx.IconSet;
  if (!pack || !pack.flat) throw new Error('icons.js did not define IconSet');
  return pack;
}

export function resolve(name) {
  const set = load();
  const real = set.flat[name] ? name : ALIASES[name];
  if (!real || !set.flat[real]) throw new Error(`Unknown icon "${name}": not in Reference/design-v2/icons.js (the pack would draw a circle)`);
  return real;
}

export function svg(name, opts = {}) {
  return load().toSVG(resolve(name), opts);
}

// One icon as a list of [tag, attrs] with themeable colours (see the header).
export function elements(name) {
  const out = [];
  const markup = svg(name, { primary: 'currentColor', secondary: SECONDARY });
  for (const m of markup.matchAll(/<(path|circle|rect|line|polyline|polygon|ellipse)\s([^>]*?)\/>/g)) {
    const attrs = {};
    for (const a of m[2].matchAll(/([a-zA-Z-]+)="([^"]*)"/g)) attrs[a[1]] = a[2];
    if (attrs.fill === SECONDARY && attrs.stroke === SECONDARY) throw new Error(`Icon ${name}: secondary fill and stroke on one element`);
    if (attrs.fill === SECONDARY) { delete attrs.fill; attrs.class = 'i2'; }
    if (attrs.stroke === SECONDARY) { delete attrs.stroke; attrs.class = 'i2s'; }
    // Knocked-out letters (the languages icon draws অ/A in white on a filled tile): the page's surface colour, so
    // they stay readable when the tile turns light in dark mode.
    if (attrs.fill === '#FFFFFF') { delete attrs.fill; attrs.class = 'ik'; }
    out.push([m[1], attrs]);
  }
  if (!out.length || JSON.stringify(out).includes(SECONDARY)) throw new Error(`Icon ${name}: could not convert the pack's SVG`);
  return out;
}

export function iconSet(names) {
  return Object.fromEntries([...new Set(names)].map((n) => [n, elements(n)]));
}

export function allNames() {
  return Object.keys(load().flat);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const outDir = path.resolve(process.argv[2] || path.join(here, 'svg'));
  fs.mkdirSync(outDir, { recursive: true });
  const names = allNames();
  for (const n of names) {
    elements(n); // validates the conversion
    fs.writeFileSync(path.join(outDir, `${n}.svg`), svg(n));
  }
  fs.writeFileSync(path.join(outDir, 'names.json'), JSON.stringify(names, null, 1));
  console.log(`icons: ${names.length} icons -> ${outDir}`);
}
