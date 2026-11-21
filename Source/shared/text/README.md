# Text library

Every piece of interface text for the desktop app, the website, and the emails lives here. English is the source language. Details and rules: `Reference/briefs/language-switcher.md`.

## Files

- `terms.json`: the product's key words. Each term has `en`, `bn.plain`, `bn.forms` (the suffixed forms its phrases need, written by a native speaker), `where`, `avoid`, and `status`.
- `phrases/<area>/<page>.json`: every sentence, by ID (`web.home.h1`). Each phrase has:
  - `en`: the English source.
  - `bn`: the Bangla text, or `""` if not written yet.
  - `status`: `draft`, `needs review`, or `reviewed`. Only `reviewed` is final.
  - `bnFrom`: the first 8 hex characters of the SHA-1 of the English text the Bangla was written from. If the English changes, the build reports the Bangla as stale and shows English until someone updates the Bangla and `bnFrom`.
- `build.mjs`: checks the files and writes the bundles.

## Term references

A phrase can use a term instead of repeating the word: `{term.library}` in English, and `{term.library}` (plain) or `{term.library:loc}` (a named form) in Bangla. Changing the term changes every phrase that uses it. If a Bangla phrase uses a term that has no Bangla yet, the phrase falls back to English.

## Build

```
node build.mjs --out <dir> [--prefix web.] [--mode ids|long] [--check]
```

Writes `en.json`, `bn.json` (Bangla, with English wherever Bangla is missing or stale), `fallback.json` (IDs showing English in Bangla), and `report.md`.

- `--mode ids` shows IDs instead of text, to find where a string comes from.
- `--mode long` stretches every string, to test layouts with long text.
- `--check` fails if any Bangla is missing or stale.

The website runs this through `node Web/Laravel/scripts/assets.mjs` (all phrases, tokens, fonts and icons).

## Where the first texts came from

The `web.*` phrases were imported from the design files in `Reference/design-archive/` (English and Bangla exactly as designed). All Bangla is marked `needs review` until a native speaker approves it. Terms taken from the design text are marked `draft`.
