// The open-source notices Likhon ships, as one text: assets/licenses/index.json lists the files and their order.
// Used by settings-ui/build.mjs (About › Open-source credits, embedded in likhon.exe) and by
// Web/Laravel/scripts/assets.mjs (public/legal/third-party-licenses.txt), so the app and the site show the same text.
// Layout: a title, a numbered list of sections, then each section under its number and title. See LICENSING.md.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const licensesDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../assets/licenses');

const WIDTH = 78;
const rule = (ch) => ch.repeat(WIDTH);

/** The notices as one text. `extra` adds sections after the listed files: [{ title, text }]. */
export function noticesText(extra = []) {
  const index = JSON.parse(fs.readFileSync(path.join(licensesDir, 'index.json'), 'utf8'));
  const listed = new Set(index.files.map((f) => f.file));
  const retired = index.retired ?? {};
  const strays = fs.readdirSync(licensesDir).filter((f) => f.endsWith('.txt') && !listed.has(f) && !(f in retired));
  if (strays.length) throw new Error(`assets/licenses: not in index.json: ${strays.join(', ')}. List each file, or retire it with a reason.`);
  for (const f of Object.keys(retired)) {
    if (fs.existsSync(path.join(licensesDir, f))) console.warn(`assets/licenses/${f} is retired (${retired[f]}).`);
  }
  const sections = [
    ...index.files.map(({ file, title }) => {
      const p = path.join(licensesDir, file);
      if (!fs.existsSync(p)) throw new Error(`assets/licenses/${file} is listed in index.json but missing`);
      return { title, text: fs.readFileSync(p, 'utf8') };
    }),
    ...extra,
  ].map((s, i) => ({ ...s, n: i + 1, text: s.text.replace(/\r\n/g, '\n').trim() }));

  const contents = sections.map(({ n, title }) => `  ${String(n).padStart(2)}. ${title}`).join('\n');
  const body = sections.map(({ n, title, text }) => `${rule('=')}\n${n}. ${title}\n${rule('=')}\n\n${text}\n`).join('\n');
  return `LIKHON: OPEN-SOURCE NOTICES\n\n${contents}\n\n${body}`;
}
