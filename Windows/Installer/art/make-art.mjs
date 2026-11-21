// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
// not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Draws the installer's brand mark: the Likhon feather in white on the accent blue, as plain 24-bit .bmp files
// (Inno Setup's script can load .bmp without any plugin). Run:  node Desktop/Installer/art/make-art.mjs
// Output: mark-44.bmp, mark-66.bmp, mark-88.bmp (100%, 150% and 200% DPI) beside this file. Dependency free.
// The feather path is the single path of Reference/design-v2/assets/likhon-mark.svg (viewBox 1200 x 1200).
// Ground colour #2747C4 is the design system's `accent`; the sidebar panel in likhon.iss uses the same value.
import { writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const GROUND = [0x27, 0x47, 0xc4];
const INK = [255, 255, 255];
const D = 'M961.5,200c-111.9,7-229.7,44-319.6,96c-14,8-25,19-32,33l-34,72v-72c-123.8,69-240.7,179-285.6,299c-24,64-18,128,10,176  c89.9-136,239.7-299,410.5-407c-150.8,123-310.6,323-480.4,603l74.9-29c23-43,46.9-84,77.9-125c176.8,4,336.6-146,412.5-269L606,621  c93.9-29,173.8-61,219.7-113c53.9-60,113.9-178,143.8-296C972,202,969.5,199.5,961.5,200z';

// Path to a flat polygon: supports the commands this path uses (M L l v h c C z).
function flatten(d) {
  const tok = d.match(/[a-zA-Z]|-?\d*\.?\d+(?:e-?\d+)?/g);
  let i = 0, cmd = '', x = 0, y = 0, sx = 0, sy = 0;
  const pts = [];
  const num = () => parseFloat(tok[i++]);
  const cubic = (x1, y1, x2, y2, x3, y3) => {
    for (let t = 1; t <= 16; t++) {
      const u = t / 16, a = (1 - u) ** 3, b = 3 * u * (1 - u) ** 2, c = 3 * u * u * (1 - u), e = u ** 3;
      pts.push([a * x + b * x1 + c * x2 + e * x3, a * y + b * y1 + c * y2 + e * y3]);
    }
    x = x3; y = y3;
  };
  while (i < tok.length) {
    if (/[a-zA-Z]/.test(tok[i])) cmd = tok[i++];
    switch (cmd) {
      case 'M': x = num(); y = num(); sx = x; sy = y; pts.push([x, y]); cmd = 'L'; break;
      case 'L': x = num(); y = num(); pts.push([x, y]); break;
      case 'l': x += num(); y += num(); pts.push([x, y]); break;
      case 'v': y += num(); pts.push([x, y]); break;
      case 'h': x += num(); pts.push([x, y]); break;
      case 'c': { const a = [num(), num(), num(), num(), num(), num()]; cubic(x + a[0], y + a[1], x + a[2], y + a[3], x + a[4], y + a[5]); break; }
      case 'C': { const a = [num(), num(), num(), num(), num(), num()]; cubic(a[0], a[1], a[2], a[3], a[4], a[5]); break; }
      case 'z': case 'Z': x = sx; y = sy; break;
      default: throw new Error('unsupported path command ' + cmd);
    }
  }
  return pts;
}

const poly = flatten(D);
const xs = poly.map((p) => p[0]), ys = poly.map((p) => p[1]);
const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];

function inside(px, py) { // non-zero winding, the same fill rule as the SVG default
  let wn = 0;
  for (let k = 0, n = poly.length; k < n; k++) {
    const [ax, ay] = poly[k], [bx, by] = poly[(k + 1) % n];
    if (ay <= py) { if (by > py && (bx - ax) * (py - ay) - (px - ax) * (by - ay) > 0) wn++; }
    else if (by <= py && (bx - ax) * (py - ay) - (px - ax) * (by - ay) < 0) wn--;
  }
  return wn !== 0;
}

function render(size) {
  const pad = size * 0.1, box = size - 2 * pad;
  const k = box / Math.max(x1 - x0, y1 - y0);
  const ox = pad + (box - (x1 - x0) * k) / 2, oy = pad + (box - (y1 - y0) * k) / 2;
  const SS = 4, rows = [];
  for (let y = 0; y < size; y++) {
    const row = [];
    for (let x = 0; x < size; x++) {
      let hit = 0;
      for (let sy = 0; sy < SS; sy++) for (let sx = 0; sx < SS; sx++) {
        const u = x + (sx + 0.5) / SS, v = y + (sy + 0.5) / SS;
        if (inside((u - ox) / k + x0, (v - oy) / k + y0)) hit++;
      }
      const a = hit / (SS * SS);
      row.push(GROUND.map((g, c) => Math.round(g + (INK[c] - g) * a)));
    }
    rows.push(row);
  }
  return rows;
}

function bmp(rows) {
  const h = rows.length, w = rows[0].length, stride = (w * 3 + 3) & ~3;
  const buf = Buffer.alloc(54 + stride * h);
  buf.write('BM', 0); buf.writeUInt32LE(buf.length, 2); buf.writeUInt32LE(54, 10);
  buf.writeUInt32LE(40, 14); buf.writeInt32LE(w, 18); buf.writeInt32LE(h, 22);
  buf.writeUInt16LE(1, 26); buf.writeUInt16LE(24, 28); buf.writeUInt32LE(stride * h, 34);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const [r, g, b] = rows[h - 1 - y][x], o = 54 + y * stride + x * 3;
    buf[o] = b; buf[o + 1] = g; buf[o + 2] = r;
  }
  return buf;
}

for (const size of [44, 66, 88]) {
  writeFileSync(join(HERE, `mark-${size}.bmp`), bmp(render(size)));
  console.log(`mark-${size}.bmp`);
}
