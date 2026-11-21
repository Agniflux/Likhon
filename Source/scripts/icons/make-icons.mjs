// Builds the exe's icons from the v2 design (Reference/design-v2/Likhon Desktop.dc.html §2) into
// Desktop/Source/assets/icons/. Build-time only: run `npm install && npm run build` here after the mark,
// the icon pack or the tokens change. Replaces the old System.Drawing script (scripts/make-icons.ps1).
//
//   likhon.ico             app icon: the white quill on an accent tile, 16 to 256 px. At 16 and 20 px the quill is
//                          enlarged to 84% of the tile and the corners tighten (design note in §2).
//   tray-<state>-<theme>.ico  tray icons from the pack: bn (letter-bangla-o, accent), en (letter-a, muted),
//                          paused (pause-circle, amber pair), error (alert-circle, red pair); for light and dark
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

const png = (svgText, size) => new Resvg(svgText, { fitTo: { mode: 'width', value: size }, background: 'rgba(0,0,0,0)' }).render().asPng();

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

// Tray states. Colours come from the tokens: pack primary/secondary as the design shows them in §2.
const STATES = {
  bn: { icon: 'letter-bangla-o', light: ['accent'], dark: ['accent'] },
  en: { icon: 'letter-a', light: ['muted'], dark: ['muted'] },
  paused: { icon: 'pause-circle', light: ['amber', 'amberSoft'], dark: ['amber', 'amberSoft'] },
  error: { icon: 'alert-circle', light: ['red', 'redSoft'], dark: ['red', 'redSoft'] },
};
for (const [state, spec] of Object.entries(STATES)) {
  for (const theme of ['light', 'dark']) {
    const [p, s] = spec[theme];
    const opts = { primary: tokens[p][theme], secondary: s ? tokens[s][theme] : tokens[p][theme] };
    // Single-colour glyphs (bn, en) have no secondary parts; give the secondary the primary colour anyway.
    made.push(write(`tray-${state}-${theme}.ico`, [16, 20, 24, 32, 40, 48, 64], (size) => packSvg(spec.icon, { ...opts, size })));
  }
}
console.log('icons:\n  ' + made.join('\n  '));
