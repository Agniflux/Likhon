# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
"""Builds the dictionary Likhon ships (assets/dict/mpl/words.sqlite): the curated Avro word list plus a larger list
of added words. Build-time only.

    python -I build-dictionary.py <avro.sqlite> <enriched.sqlite> [<out.sqlite>]

<avro.sqlite> is the Avro-only dictionary made by scripts/avro/convert-dictionary.mjs (159,380 words, suffixes and the
phonetic autocorrect table, MPL-2.0). <enriched.sqlite> is a likhon-dictionary file with more words (owner-supplied,
2026-10-23: words_enriched_merged.sqlite, 843,863 words, which include the Avro ones). The output has:
  - words:       the Avro words, as they are (suggestions offer these first);
  - more:        the added words (spelling checks and suggestions after the Avro ones), except entries whose letters
                 cannot form a Bangla word: a vowel sign or hasant at the start, two vowel signs together, a vowel sign
                 after a full vowel, a hasant before a vowel, the same letter three times running, digits mixed with
                 letters, or more than 30 characters (run-together phrases);
  - suffixes, autocorrect: from the Avro file; meta: format, version 1, licence and counts.
It holds MPL-2.0 data, so it is MPL-2.0 and stays in assets/dict/mpl (SOURCE.md there).
"""
import os
import re
import sqlite3
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
# The enlarged dictionary is local only (it holds the owner's own word list): words.full.sqlite, never in git. The public
# words.sqlite is the Avro-derived part of it (the `more` table dropped), made by scripts/dict/make-public-dictionary.py.
OUT = os.path.join(HERE, '..', '..', 'assets', 'dict', 'mpl', 'words.full.sqlite')

SIGN = 'া-ৄেৈোৌৗৢৣ'
MARK = 'ঁ-ঃ়্'
RULES = [
    ('long', lambda w: len(w) > 30),
    ('same letter three times', lambda w: re.search(r'(.)\1\1', w)),
    ('starts with a sign', lambda w: re.match(f'[{SIGN}{MARK}]', w)),
    ('two vowel signs', lambda w: re.search(f'[{SIGN}][{SIGN}]', w)),
    ('vowel sign after a vowel', lambda w: re.search(f'[অ-ঔ][{SIGN}]', w)),
    ('misplaced hasant', lambda w: re.search('্্|্[অ-ঔা-ৌ]', w)),
    ('digits and letters', lambda w: re.search('[০-৯]', w) and re.search('[অ-ড়]', w)),
]


def is_bangla(w):
    return w and all('ঀ' <= c <= '৿' or c in '‌‍' for c in w)


def ro(path):
    return sqlite3.connect(f'file:{os.path.abspath(path)}?mode=ro', uri=True)


def main():
    avro_path, enriched_path = sys.argv[1], sys.argv[2]
    out = sys.argv[3] if len(sys.argv) > 3 else OUT
    avro = ro(avro_path)
    base = {r[0] for r in avro.execute('SELECT word FROM words')}
    suffixes = list(avro.execute('SELECT roman, bangla FROM suffixes'))
    autocorrect = list(avro.execute('SELECT roman, phonetic FROM autocorrect'))
    avro.close()
    more, dropped = set(), {}
    enriched = ro(enriched_path)
    for (w,) in enriched.execute('SELECT word FROM words'):
        if w in base or w in more:
            continue
        reason = 'not Bangla' if not is_bangla(w) else next((name for name, test in RULES if test(w)), None)
        if reason:
            dropped[reason] = dropped.get(reason, 0) + 1
        else:
            more.add(w)
    enriched.close()

    tmp = out + '.tmp'
    if os.path.exists(tmp):
        os.remove(tmp)
    db = sqlite3.connect(tmp)
    db.executescript("""
        CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID;
        CREATE TABLE words (word TEXT PRIMARY KEY) WITHOUT ROWID;
        CREATE TABLE more (word TEXT PRIMARY KEY) WITHOUT ROWID;
        CREATE TABLE suffixes (roman TEXT PRIMARY KEY, bangla TEXT NOT NULL) WITHOUT ROWID;
        CREATE TABLE autocorrect (roman TEXT PRIMARY KEY, phonetic TEXT NOT NULL) WITHOUT ROWID;
    """)
    db.executemany('INSERT INTO words VALUES (?)', ((w,) for w in sorted(base)))
    db.executemany('INSERT INTO more VALUES (?)', ((w,) for w in sorted(more)))
    db.executemany('INSERT INTO suffixes VALUES (?, ?)', suffixes)
    db.executemany('INSERT INTO autocorrect VALUES (?, ?)', autocorrect)
    meta = {'format': 'likhon-dictionary', 'version': '1', 'license': 'MPL-2.0', 'words': str(len(base) + len(more)),
            'suffixes': str(len(suffixes)), 'autocorrect': str(len(autocorrect))}
    db.executemany('INSERT INTO meta VALUES (?, ?)', meta.items())
    db.commit()
    db.execute('VACUUM')
    db.close()
    os.replace(tmp, out)
    print(f'{os.path.normpath(out)}: {len(base)} curated + {len(more)} added words, {len(suffixes)} suffixes, {len(autocorrect)} autocorrect')
    for k, v in sorted(dropped.items(), key=lambda kv: -kv[1]):
        print(f'  left out, {k}: {v}')


main()
