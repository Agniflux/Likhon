"""Converts a Windows keyboard layout source (.klc, Microsoft Keyboard Layout Creator) into a Likhon layout JSON file
(shared/schemas/layout.schema.json). Build-time only.

    python -I convert-klc.py <source.klc> <out.json> <id> <name-en> <name-bn> <author> <license> [description]

Levels: column 0 normal, 1 Shift, 6 AltGr (Ctrl+Alt), 7 Shift+AltGr. "%%" takes the LIGATURE row, "@" (a dead key)
types the character itself; the engine turns hasant + vowel sign into the full vowel, which is what UniJoy's dead
hasant does. Keys outside the schema's key names (the extra 102nd key, the numpad) are left out.
"""
import json
import sys

VK = {'OEM_3': '`', 'OEM_MINUS': '-', 'OEM_PLUS': '=', 'OEM_4': '[', 'OEM_6': ']', 'OEM_5': '\\', 'OEM_1': ';',
      'OEM_7': "'", 'OEM_COMMA': ',', 'OEM_PERIOD': '.', 'OEM_2': '/'}
LEVELS = {0: 'normal', 1: 'shift', 6: 'altgr', 7: 'shiftAltgr'}


def main():
    src, out, lid, name_en, name_bn, author, lic = sys.argv[1:8]
    desc = sys.argv[8] if len(sys.argv) > 8 else None
    raw = open(src, 'rb').read()
    text = raw.decode('utf-16') if raw[:2] in (b'\xff\xfe', b'\xfe\xff') else raw.decode('utf-8')
    lines = [l.split('//')[0].rstrip() for l in text.splitlines()]

    def section(name):
        i = next(i for i, l in enumerate(lines) if l.strip().split('\t')[0] == name)
        rows = []
        for l in lines[i + 1:]:
            if l and not l[0].isspace() and l.split()[0].isupper() and l.split()[0] not in VK and len(l.split()[0]) > 3 and not l.split()[0].startswith('OEM'):
                break
            if l.strip():
                rows.append(l.split())
        return rows

    shiftstates = [int(r[0]) for r in section('SHIFTSTATE')]
    ligatures = {}
    for r in section('LIGATURE'):
        if len(r) >= 3 and not r[0].startswith('/'):
            ligatures[(r[0], int(r[1]))] = ''.join(chr(int(c, 16)) for c in r[2:])

    keys = {}
    for r in section('LAYOUT'):
        if len(r) < 3 + len(shiftstates) or not all(c in '0123456789abcdef' for c in r[0].lower()):
            continue
        vk = r[1]
        key = VK.get(vk, vk.lower() if len(vk) == 1 else None)
        if key is None:
            continue
        levels = {}
        for col, state in enumerate(shiftstates):
            if state not in LEVELS:
                continue
            v = r[3 + col]
            if v == '-1':
                continue
            if v == '%%':
                s = ligatures.get((vk, col))
            else:
                v = v.rstrip('@')
                s = chr(int(v, 16)) if len(v) == 4 and all(c in '0123456789abcdefABCDEF' for c in v) else v
            if s:
                levels[LEVELS[state]] = s
        if levels:
            keys[key] = levels

    layout = {'format': 'likhon-layout', 'version': 1, 'id': lid, 'name': {'en': name_en, 'bn': name_bn}, 'kind': 'fixed',
              'author': author}
    if desc:
        layout['description'] = desc
    layout['license'] = lic
    layout['keys'] = dict(sorted(keys.items()))
    layout['numpad'] = {str(d): chr(0x09E6 + d) for d in range(10)} | {'+': '+', '.': '.', '/': '/', '*': '*', '-': '-'}
    with open(out, 'w', encoding='utf-8', newline='\n') as f:
        json.dump(layout, f, ensure_ascii=False, indent=1)
        f.write('\n')
    print(f'{out}: {len(keys)} keys')


main()
