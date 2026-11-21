// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Builds the exe's icons from the v2 design (Reference/design-v2/Likhon Desktop.dc.html §2) into
// Desktop/Source/assets/icons/. Build-time only: run `npm install && npm run build` here after the mark,
// the icon pack or the tokens change. Replaces the old System.Drawing script (scripts/make-icons.ps1).
//
//   likhon.ico             app icon: the white quill on an accent tile, 16 to 256 px. At 16 and 20 px the quill is
//                          enlarged to 84% of the tile and the corners tighten (design note in §2).
//   tray-<state>-<theme>.ico  tray badges (design system TrayIcon): bn (অ on accent), en (A on surface2),
//                          paused (॥ on amber), error (! on red); for light and dark
//                          taskbars, 16 to 64 px so every display scale gets a sharp frame.
//
// Every frame is a PNG inside the .ico (supported by Windows Vista and later).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Resvg } from '@resvg/resvg-js';
import { svg as packSvg, QUILL } from '../../shared/icons/build-icons.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const out = path.resolve(here, '../../assets/icons');
const tokens = JSON.parse(fs.readFileSync(path.resolve(here, '../../../Reference/design-tokens.json'), 'utf8')).color;
fs.mkdirSync(out, { recursive: true });

const mark = (size) => {
  const small = size <= 20;
  const rx = small ? 181.8181818181818 : 270;
  const g = small ? 'translate(-93.33333333333334,-93.33333333333334) scale(1.1555555555555554)' : 'translate(190,190) scale(0.9111) translate(-150,-150)';
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 1200 1200"><rect width="1200" height="1200" rx="${rx}" fill="${tokens.accent.light}"/><g transform="${g}"><path fill="#FFFFFF" d="${QUILL}"/></g></svg>`;
};

// Hind Siliguri SemiBold draws the tray letters A and অ (owner, 2026-11-21); the exe's UI font, so the badge matches the app.
const FONT = path.resolve(here, '../../assets/fonts/HindSiliguri-SemiBold.ttf');
const png = (svgText, size) => new Resvg(svgText, { fitTo: { mode: 'width', value: size }, background: 'rgba(0,0,0,0)', font: { fontFiles: [FONT], loadSystemFonts: false, defaultFontFamily: 'Hind Siliguri' } }).render().asPng();

function ico(frames) {
  // ICONDIR + ICONDIRENTRY per frame, then the PNG data.
  const head = Buffer.alloc(6 + 16 * frames.length);
  head.writeUInt16LE(0, 0); head.writeUInt16LE(1, 2); head.writeUInt16LE(frames.length, 4);
  let offset = head.length;
  frames.forEach(({ size, data }, i) => {
    const e = 6 + 16 * i;
    head.writeUInt8(size >= 256 ? 0 : size, e); head.writeUInt8(size >= 256 ? 0 : size, e + 1);
    head.writeUInt8(0, e + 2); head.writeUInt8(0, e + 3);
    head.writeUInt16LE(1, e + 4); head.writeUInt16LE(32, e + 6);
    head.writeUInt32LE(data.length, e + 8); head.writeUInt32LE(offset, e + 12);
    offset += data.length;
  });
  return Buffer.concat([head, ...frames.map((f) => f.data)]);
}

function write(name, sizes, draw) {
  const file = path.join(out, name);
  fs.writeFileSync(file, ico(sizes.map((size) => ({ size, data: png(draw(size), size) }))));
  return `${name} ${fs.statSync(file).size} B`;
}

const made = [write('likhon.ico', [16, 20, 24, 32, 40, 48, 64, 96, 128, 256], mark)];

// Tray badges (design system component TrayIcon, 2026-11-21): a filled rounded square per state with a glyph that
// differs in shape as well as colour. Bangla on: accent tile, `on` glyph অ. English: surface2 tile, ink glyph A, a
// line border. Paused: amber tile, `bg` glyph ॥. Error: red tile, `bg` glyph !. Light and dark values from the tokens.
// অ and A are Hind Siliguri SemiBold text; ॥ and ! are drawn here as plain shapes.
const SW = 2.4; // stroke of the drawn glyphs in the 24-unit box, heavy enough to hold at 16 px
const letter = (ch, y, size) => (c) => `<text x="12" y="${y}" text-anchor="middle" font-family="Hind Siliguri" font-weight="600" font-size="${size}" fill="${c}">${ch}</text>`;
const bars = (c) => `<path d="M9 6v12M15 6v12" fill="none" stroke="${c}" stroke-width="${SW + 0.4}" stroke-linecap="round"/>`;
const bang = (c) => `<path d="M12 5.5v8" fill="none" stroke="${c}" stroke-width="${SW + 0.4}" stroke-linecap="round"/><circle cx="12" cy="18" r="1.7" fill="${c}"/>`;
const STATES = {
  bn: { tile: 'accent', glyph: 'on', scale: 1, body: letter('অ', 17.4, 18.5) },
  en: { tile: 'surface2', glyph: 'ink', border: 'line', scale: 1, body: letter('A', 17.6, 19.5) },
  paused: { tile: 'amber', glyph: 'bg', body: bars },
  error: { tile: 'red', glyph: 'bg', body: bang },
};
const badge = (spec, theme, size) => {
  const t = (k) => tokens[k][theme];
  const border = spec.border ? ` stroke="${t(spec.border)}" stroke-width="1"` : '';
  const inset = spec.border ? 0.5 : 0;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24"><rect x="${inset}" y="${inset}" width="${24 - 2 * inset}" height="${24 - 2 * inset}" rx="5" fill="${t(spec.tile)}"${border}/><g transform="translate(12 12) scale(${spec.scale ?? 0.78}) translate(-12 -12)">${spec.body(t(spec.glyph))}</g></svg>`;
};
for (const [state, spec] of Object.entries(STATES)) {
  for (const theme of ['light', 'dark']) {
    made.push(write(`tray-${state}-${theme}.ico`, [16, 20, 24, 32, 40, 48, 64], (size) => badge(spec, theme, size)));
  }
}
console.log('icons:\n  ' + made.join('\n  '));
