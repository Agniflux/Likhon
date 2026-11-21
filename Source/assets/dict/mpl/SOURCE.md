# Dictionary converted from Avro Keyboard

`words.sqlite` is converted from Avro Keyboard's word database and phonetic autocorrect list by
`scripts/avro/convert-dictionary.mjs`. Avro Keyboard is
Copyright OmicronLab and is published under the Mozilla Public License 2.0 (`LICENSE` in this folder), so this file is
covered by the same licence and stays in this folder.

Sources:
- https://github.com/mugli/Avro-Keyboard/blob/6766d27fd2c7b41d0ce77fe872f90d2f6348c3b1/assets/Database.db3
- https://github.com/mugli/Avro-Keyboard/blob/6766d27fd2c7b41d0ce77fe872f90d2f6348c3b1/assets/autodict.dct

Changes from the source: the 48 per-letter tables are merged into one sorted `words` table (Unicode NFC, duplicates and
non-Bangla rows removed), the Suffix table becomes `suffixes`, the autocorrect text file becomes the `autocorrect` table
(word rows only; emoticon rows and rows that change nothing left out), and a `meta` table names the format and licence.
Words: 159380. Suffixes: 737. Autocorrect: 2222. Rows left out: 4 words,
251 autocorrect.

## What is not in this file

Nothing else. This public file holds the Avro Keyboard words only. A larger local dictionary (extra words from a
word list the maintainers hold, with its sources still being recorded) is not part of this repository and is not
distributed from it; `scripts/dict/make-public-dictionary.py` shows how this file is made from it.
