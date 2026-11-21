"""Turns Avro Keyboard skins (.avroskin: XML holding BMP pictures of Avro's top bar buttons) into Likhon themes
(JSON, shared/schemas/theme.schema.json). Build-time only; needs Python 3 with Pillow.

    python scripts/avro/convert-skins.py <folder with the .avroskin files> <commit>

Likhon themes are colours, not pictures (owner decisions 2026-10-22: themes are JSON files only; make an approximate
colour theme from each skin). So each theme is read off the skin's pictures:
  surface  the commonest colour of the top bar background (TopBarMain)
  accent   the strongest colour across all the skin's pictures that differs from the surface
  ink      the button colour with the best contrast on the surface (else black or white)
and the rest (surface2, line, muted, sel, on, desk) are mixed from those. Contrast is then checked and corrected:
ink and on at least 4.5:1, muted at least 4.5:1, accent at least 3:1 against the surface.

Every Likhon theme has a light and a dark version (owner decision 2026-10-22). A clearly light skin gives the light
version and a clearly dark one the dark version; every other version (and both, for a mid-tone skin such as Luna's
bright blue) is made from Likhon's light or dark ground tinted with the skin's own colours.

Take the inputs from github.com/mugli/Avro-Keyboard (assets/skins) at a pinned commit. The skins are MPL-2.0, so the
themes are written to assets/themes/mpl/ beside LICENSE and SOURCE.md. Their names and ids do not mention Avro (owner
decision 2026-10-22); SOURCE.md and the About credits do.
"""
import base64
import io
import json
import re
import sys
from collections import Counter
from pathlib import Path

from PIL import Image

src, commit = Path(sys.argv[1]), sys.argv[2]
assert re.fullmatch(r'[0-9a-f]{40}', commit), 'commit: 40 hex characters'
out = Path(__file__).resolve().parents[2] / 'assets' / 'themes' / 'mpl'
out.mkdir(parents=True, exist_ok=True)
PALETTES = json.loads((Path(__file__).resolve().parent / 'skin-palettes.json').read_text(encoding='utf-8'))


def picture(xml, tag):
    m = re.search(r'<%s>([\s\S]*?)</%s>' % (tag, tag), xml)
    data = re.sub(r'\s', '', m.group(1)) if m else ''
    return Image.open(io.BytesIO(base64.b64decode(data))).convert('RGB') if data else None


def cdata(xml, tag):
    m = re.search(r'<%s>\s*<!\[CDATA\[([\s\S]*?)\]\]>' % tag, xml)
    return m.group(1).strip() if m else ''


def lum(c):
    def ch(v):
        v /= 255
        return v / 12.92 if v <= 0.03928 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = c
    return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)


def contrast(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def mix(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def sat(c):
    return max(c) - min(c)


def colours(im, n=24):
    """The commonest colours, a little quantised so gradients count as one."""
    small = im.copy()
    small.thumbnail((120, 40))
    q = Counter(tuple(v // 8 * 8 + 4 for v in px) for px in small.getdata())
    return [c for c, _ in q.most_common(n)]


def push(c, against, need, towards):
    """Moves c towards black or white until it has `need` contrast against `against`."""
    t = 0.0
    while contrast(c, against) < need and t < 1:
        t += 0.05
        c = mix(c, towards, t)
    return c


def derived(accent, dark, tint):
    """A version made from Likhon's own light or dark ground, tinted with the skin's bar colour and accent."""
    if dark:
        base, ink0, muted0, desk = (0x1C, 0x21, 0x2C), (0xE8, 0xEB, 0xF2), (0xA3, 0xAB, 0xBA), (0x10, 0x14, 0x1A)
    else:
        base, ink0, muted0, desk = (0xFF, 0xFF, 0xFF), (0x1B, 0x22, 0x30), (0x5B, 0x64, 0x74), (0xDC, 0xE2, 0xE8)
    # A grey bar adds nothing but dimness, so only a coloured one tints the ground.
    surface = mix(mix(base, tint, 0.10) if sat(tint) >= 40 else base, accent, 0.04)
    acc = push(accent, surface, 3.0, (255, 255, 255) if dark else (0, 0, 0))
    on = (0, 0, 0) if contrast((0, 0, 0), acc) >= contrast((255, 255, 255), acc) else (255, 255, 255)
    return {
        'desk': hexc(mix(desk, accent, 0.12)),
        'surface': hexc(surface),
        'surface2': hexc(mix(surface, ink0, 0.06)),
        'line': hexc(mix(surface, ink0, 0.18)),
        'ink': hexc(push(ink0, surface, 7.0, (255, 255, 255) if dark else (0, 0, 0))),
        'muted': hexc(push(muted0, surface, 4.5, (255, 255, 255) if dark else (0, 0, 0))),
        'accent': hexc(acc),
        'on': hexc(on),
        'sel': hexc(mix(surface, accent, 0.22)),
    }


def hexc(c):
    return '#%02X%02X%02X' % c


# Plain names (owner decision 2026-10-22: no theme is named after Avro): skin file name -> (id, English, Bangla).
NAMES = {
    'Aero7 dark': ('aero', 'Aero', 'এরো'),
    'Avro Classic': ('classic', 'Classic', 'ক্লাসিক'),
    'Avro Flat (cyan)': ('flat-cyan', 'Flat cyan', 'ফ্ল্যাট সায়ান'),
    'Avro Flat (Mint)': ('flat-mint', 'Flat mint', 'ফ্ল্যাট মিন্ট'),
    'Avro Flat (Red)': ('flat-red', 'Flat red', 'ফ্ল্যাট লাল'),
    'Avro Flat (Yellow)': ('flat-yellow', 'Flat yellow', 'ফ্ল্যাট হলুদ'),
    'iAvro': ('maroon', 'Maroon', 'মেরুন'),
    'Avro Luna': ('luna', 'Luna', 'লুনা'),
    'Avro Royal': ('royal', 'Royal', 'রয়্যাল'),
    'Avro Vista 2': ('vista-2', 'Vista 2', 'ভিস্তা ২'),
    'Avro Vista': ('vista', 'Vista', 'ভিস্তা'),
}


rows = []
for f in sorted(src.glob('*.avroskin')):
    xml = f.read_text(encoding='utf-8-sig')
    name = cdata(xml, 'SkinName') or f.stem
    bar = picture(xml, 'TopBarMain')
    buttons = [p for p in (picture(xml, t) for t in ('KeyboardModeBanglaNormal', 'KeyboardModeEnglishNormal', 'KeyboardLayoutNormal', 'ToolsNormal')) if p]
    every = [p for p in (picture(xml, t) for t in sorted(set(re.findall(r'<([A-Za-z]+(?:Normal|Over|Down|Main))>', xml)))) if p]

    surface = colours(bar, 1)[0]
    dark = lum(surface) < 0.18
    # Only a clearly light or clearly dark bar is used as it is; a mid-tone bar only tints both versions.
    own_light, own_dark = lum(surface) >= 0.55, lum(surface) <= 0.08
    black, white = (0, 0, 0), (255, 255, 255)
    far = white if dark else black

    seen = Counter(c for p in every for c in colours(p, 40))
    strong = [c for c, n in seen.items() if sat(c) >= 60 and contrast(c, surface) > 1.3]
    accent = max(strong, key=lambda c: (seen[c] ** 0.5) * sat(c), default=None)
    if accent is None:
        accent = (0x8F, 0xA6, 0xFF) if dark else (0x27, 0x47, 0xC4)  # the Likhon accent when a skin has no colour of its own
    accent = push(accent, surface, 3.0, far)

    ink = max((c for p in buttons for c in colours(p)), key=lambda c: contrast(c, surface), default=far)
    ink = push(ink, surface, 7.0, far)
    muted = push(mix(surface, ink, 0.6), surface, 4.5, far)
    on = black if contrast(black, accent) >= contrast(white, accent) else white
    own = {
        'desk': hexc(mix(surface, black, 0.35) if not dark else mix(surface, black, 0.4)),
        'surface': hexc(surface),
        'surface2': hexc(mix(surface, ink, 0.06)),
        'line': hexc(mix(surface, ink, 0.18)),
        'ink': hexc(ink),
        'muted': hexc(muted),
        'accent': hexc(accent),
        'on': hexc(on),
        'sel': hexc(mix(surface, accent, 0.22)),
    }
    theme = {
        'format': 'likhon-theme', 'version': 1, 'id': NAMES[f.stem][0], 'name': {'en': NAMES[f.stem][1], 'bn': NAMES[f.stem][2]},
        'author': (cdata(xml, 'DesignerName') or 'OmicronLab').split(',')[0].strip(),  # the name only, no contact details 'license': 'MPL-2.0',
        'light': own if own_light else derived(accent, False, surface),
        'dark': own if own_dark else derived(accent, True, surface),
    }
    # The curated palette for this skin wins over the automatic guess (skin-palettes.json; owner, 2026-10-23: match
    # the original skins as closely as possible).
    for mode in ('light', 'dark'):
        theme[mode] = PALETTES.get(theme['id'], {}).get(mode, theme[mode])
    (out / (theme['id'] + '.json')).write_text(json.dumps(theme, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    rows.append('| `%s.json` | %s | %s |' % (theme['id'], f.name, theme['author']))
    print('%-28s dark=%-5s surface %s accent %s ink %s' % (theme['id'], dark, hexc(surface), hexc(accent), hexc(ink)))

(out / 'SOURCE.md').write_text(f"""# Themes made from Avro Keyboard skins

These themes are made from Avro Keyboard's skins by `scripts/avro/convert-skins.py`. Avro Keyboard is Copyright
OmicronLab and is published under the Mozilla Public License 2.0 (`LICENSE` in this folder), so these files are covered
by the same licence and stay in this folder.

Source: https://github.com/mugli/Avro-Keyboard/tree/{commit}/assets/skins

The skins are pictures; Likhon themes are colours. Each theme takes its bar, accent and text colours from the skin's
pictures and mixes the rest, then corrects contrast (text 4.5:1 or more, accent 3:1). They are approximations, not
copies of the skins' look.

| Likhon file | Avro skin | Designer (as the skin names them) |
| --- | --- | --- |
""" + '\n'.join(rows) + '\n', encoding='utf-8')
