# Emoji names

`emoji.tsv` (language, name, emoji) is made from the Unicode CLDR emoji annotations, English and Bangla
(npm `cldr-annotations-full` 48.2.0, `annotations/en/annotations.json` and `annotations/bn/annotations.json`), under
the Unicode License v3 (`LICENSE`). Only one-word names are kept (a name is typed between colons, and Space ends a
word); a name used by several emoji goes to the first emoji in CLDR order, its spoken name (tts) first. One change:
English `heart` gives ❤️ (CLDR's first match was 🥰).
