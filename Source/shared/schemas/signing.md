# Signing and the signed files (proposal)

Shared contract between the desktop app, the website, and the server (root `CLAUDE.md`, sections 5e and 5f).
Owner decision 2026-10-13: **Ed25519**. Everything else here is a proposal until the desktop side (M1, M7) confirms it.

## Keys

- One Ed25519 key pair. The **private key stays offline with the owner**: never in the repository, on the web server, or in CI logs.
- The **public key** (32 bytes) is embedded in `likhon.exe` and set on the server as `LIKHON_SIGNING_PUBLIC_KEY` (base64).

## Signature files

- A detached signature over the exact bytes of the file.
- Stored next to the file as `<file>.sig`: the 64-byte signature, **base64, one line**, no newline required.
- Verifiers reject a file whose signature does not verify, or which has no `.sig`.

## Signed files

| File | What | Who reads it |
| --- | --- | --- |
| `library/items/<id>/<version>/<file>` | A library pack (layout, theme, list, font zip) | App, when the person adds it |
| `library/index.json` | The library index | App (when the person opens the library), website build |
| `library/revoked.json` | The revoke list | App, when the person checks for updates |
| `releases/<version>/<file>` | Portable exe, setup programs | App updater, Download page |
| `releases/<version>/manifest.json` | The update manifest | App, when the person checks for updates |

### `library/index.json`

```json
{
  "format": "likhon-library-index",
  "version": 12,
  "generated": "2026-10-13T10:00:00Z",
  "items": [
    {
      "id": "easy-probhat-plus",
      "type": "layout",
      "name": { "en": "Easy Probhat Plus", "bn": "সহজ প্রভাত প্লাস" },
      "desc": { "en": "…", "bn": "…" },
      "tags": ["phonetic"],
      "author": "Author name",
      "license": "CC-BY-4.0",
      "version": "1.1",
      "file": "items/easy-probhat-plus/1.1/easy-probhat-plus.zip",
      "size": 6144,
      "sha256": "…",
      "sideload": true,
      "published": "2026-10-13T10:00:00Z"
    }
  ]
}
```

`version` only goes up. The app refuses an index older than the one it already has.

### `library/revoked.json`

```json
{
  "format": "likhon-revoke-list",
  "version": 4,
  "generated": "2026-10-13T10:00:00Z",
  "revoked": [ { "id": "medical-terms", "versions": ["1.2"], "at": "2026-10-12T08:00:00Z" } ]
}
```

No reasons or personal data: the app only needs to know what to turn off.

### `releases/<version>/manifest.json` (proposal, 2026-10-25)

Written by `Desktop/Signer/ed25519/likhon-sign.mjs manifest`:

```json
{
  "format": "likhon-update-manifest",
  "version": "0.1.2",
  "generated": "2026-10-25T18:00:00Z",
  "files": [ { "kind": "portable", "name": "likhon.exe", "size": 5088072, "sha256": "…" } ]
}
```

`kind` is `portable` or `setup`. The app (M7) downloads the file named here and checks its size, SHA-256 and
`.sig` before running the update helper.

## Keys in use

The owner's key pair was made on 2026-10-25 (`Desktop/Signer/ed25519/`). Public key, base64:
`w9M7IjWbe3d5Nushr/fw/L2vJt3FjggqkvFuJV0ZqTs=`

## How a batch is signed (server side)

1. An admin exports a batch from **Ready to sign**: a zip with the approved files, the proposed `library/index.json` and `library/revoked.json`, and `batch.json` listing every file and its SHA-256.
2. The owner signs every listed file offline and returns the signatures as a zip of `.sig` files at the same paths (or as one `signatures.json`: `{ "path": "base64 signature" }`).
3. The server verifies every signature against `LIKHON_SIGNING_PUBLIC_KEY` and shows the result. Only a fully verified batch can be published.
4. **Publish** copies the exact signed bytes and their `.sig` files to the public storage and marks the items published.

A revocation reaches the app once a signed revoke list containing it is published.
