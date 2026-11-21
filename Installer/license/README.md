# Installer licence agreement

`eula.en.txt` and `eula.bn.txt` are what the installer's licence page shows (English and Bangla, switched with the
installer's language). They are **generated**, not edited by hand:

```
node Desktop/Installer/license/make-eula.mjs
```

The source is the website's agreement, `Web/Laravel/resources/legal/{en,bn}/license.md` (the page at `/en/legal/license`),
so the installer and the site always say the same thing. The script drops the front matter and the `[!LAWYER]` review
notes, keeps link text, and writes plain text with CRLF line ends and a UTF-8 byte-order mark. Run it after any change
to those two Markdown files, then rebuild the installer.

## Status (2026-11-21)

- Written from the product's actual behaviour by an AI assistant, **not a lawyer**. The `[!LAWYER]` notes at the
  end of `license.md` list what a lawyer must check; the main ones: governing law (Bangladesh) and courts (Dhaka),
  the full exclusion of liability under Bangladesh consumer law, and whether the keyboard hook or text service needs
  its own consent.
- 2026-11-21: the sentence "you may not sell Likhon" was removed, because Likhon's code is MPL-2.0, which allows
  selling copies; the name and logo are protected instead (section 1 and 5). Bracketed facts were filled in as
  Bangladesh, Dhaka, and English prevailing; the owner should confirm them.
- The agreement says it covers "the Likhon app for Windows". The Mac and Linux versions have no agreement of their own
  yet: decide whether this one should be widened.
- The Bangla text is a draft marked needs review.
- Third-party licence texts are not in this file: the installer's page links to them and the app shows them under
  Settings > About (`LICENSING.md` at the root).
