"""Makes the public dictionary: assets/dict/mpl/words.sqlite from words.full.sqlite (the owner's enlarged list, local only).
The public file keeps the Avro-derived curated words, suffixes and autocorrect, and drops the `more` table, so the repository
holds only what has a known source and licence (MPL-2.0, Avro Keyboard). Run after build-dictionary.py."""
import os
import shutil
import sqlite3

d = r'D:\laragon\www\Likhon\Desktop\Source\assets\dict\mpl'
full = os.path.join(d, 'words.full.sqlite')
pub = os.path.join(d, 'words.sqlite')

if not os.path.exists(full):
    shutil.move(pub, full)  # the current words.sqlite is the full one

if os.path.exists(pub):
    os.remove(pub)
shutil.copyfile(full, pub)
c = sqlite3.connect(pub)
c.execute('DROP TABLE IF EXISTS more')
n = c.execute('SELECT count(*) FROM words').fetchone()[0]
c.execute("UPDATE meta SET value = ? WHERE key = 'words'", (str(n),))
c.commit()
c.execute('VACUUM')
c.close()

c = sqlite3.connect(pub)
print('public words.sqlite:', os.path.getsize(pub), 'bytes;',
      {t: c.execute(f'select count(*) from {t}').fetchone()[0] for t in ('words', 'suffixes', 'autocorrect')},
      dict(c.execute('select key,value from meta').fetchall()))
print('full words.full.sqlite:', os.path.getsize(full), 'bytes')
