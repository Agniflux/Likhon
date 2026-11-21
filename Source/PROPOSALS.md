# Proposals for owner decision (v2 brief §4 "To decide")

The v2 brief asks for options before building the input method, the native UI, the update channel and the package format. Each section gives options, a recommendation, and what's already built. Nothing here is decided until the owner says so.

## 1. Input method

| Option | What | For | Against |
| --- | --- | --- | --- |
| **A. Hook + TSF (recommended)** | Portable types through the low-level keyboard hook in `likhon.exe` (built in M2). The Native add-on adds a thin TSF text-service DLL (x64 and x86) that shows in Win+Space and uses the same engine. | Portable needs no admin and no install. Native gets real composition and works where hooks don't. One engine for both. | Two typing paths to test. Both share the hook/TSF handoff rule (`Desktop/CLAUDE.md` §2). |
| B. TSF only | Every edition registers the DLL. | One path. | Registering needs an install (per-user without admin is **unverified**), so it can't be Portable. |
| C. Hook only | No DLL. | Simplest. | No Win+Space entry, so §16 "Add Likhon keyboards to Windows" can't be met. |

Measured so far (SPIKES.md, M2): hook handling has a median of 152 µs per key and p99 of 279 µs over a small sample. The TSF spike (M3) is not started.

## 2. Native UI (typing-time surfaces)

| Option | What | For | Against |
| --- | --- | --- | --- |
| **A. Rust + Direct2D/DirectWrite for typing surfaces, web view for Settings (recommended, partly built)** | Tray, top bar, suggestion window, caret badge, toasts, splash, keyboard and popups are no-activate Win32 windows drawn with Direct2D and DirectWrite. Settings (§7) is a separate process using wry + tao over WebView2 (built 2026-10-13). | Never takes focus. DirectWrite shapes Bangla conjuncts. Typing never depends on WebView2. The exe is 1.32 MB today. | Every native surface is hand-drawn. Duotone icons need a path renderer (`topbar.rs` already parses SVG paths). |
| B. Web view for everything | All surfaces in WebView2. | One UI stack. | Focus and no-activate behaviour is hard to guarantee. Memory cost at idle. Typing would depend on WebView2. |
| C. WinUI 3 / Windows App SDK | XAML surfaces. | Native look. | Needs a runtime and packaging, a much larger download, and C# or C++/WinRT beside Rust. |

## 3. Update channel

| Option | What | For | Against |
| --- | --- | --- | --- |
| **A. Signed static manifest on likhon.app (recommended)** | `https://likhon.app/updates/manifest.json` plus a detached Ed25519 signature, read **only when the person taps Check for updates**. It lists the version, per-edition file URLs, SHA-256 and size, notes titles (en/bn) and the revoke-list URL. The app verifies the signature, downloads, verifies the hash, then runs `likhon.exe --update`. | No server logic, so a CDN can serve it. Nothing per-person is recorded. Matches the root CLAUDE.md "no background network". | The owner signs each release offline (already the plan). |
| B. GitHub Releases API | The app reads the releases JSON. | Free hosting. | Third-party host, rate limits. Release notes aren't bilingual by default. Still needs our own signature. |
| C. Background auto-update | Periodic checks. | Users stay current. | **Not allowed:** breaks "no network unless the person acts". |

## 4. Package format

| Option | What | For | Against |
| --- | --- | --- | --- |
| **A. Zip + `manifest.json` + detached Ed25519 `.sig` (recommended, schemas exist)** | One format for every library type (`shared/schemas`, `crates/likhon-packs`). The strict parser has size caps, no executable content and a licence from an allowlist. The signature covers the whole zip's SHA-256. The validator is a Rust library, a CLI (run by Laravel on uploads) and WASM (website designers). | Already started. One validator on three surfaces. Easy for reviewers to inspect. | Zip parsing must be hardened (zip bombs, path traversal): caps and fuzz tests are required. |
| B. Single JSON file per pack | No container. | Simplest to parse. | Fonts and multi-file packs don't fit. Base64 bloat. |
| C. MSIX / signed Windows packages | OS-level packaging. | Windows trust chain. | Heavy, needs a code-signing certificate per pack. Not usable on the website. |

**Signature scheme:** Ed25519 (root CLAUDE.md §5e). The public key is embedded in the exe. The private key stays offline with the owner. The scheme is still listed as an open decision in root CLAUDE.md §7, so confirm it here.

## Decided (owner, 2026-10-21)

1. **Input method: A.** The hook for Portable, plus a TSF DLL for the Native add-on.
2. **Native UI: A.** Rust with Direct2D/DirectWrite for typing-time surfaces, and wry + tao (WebView2) for Settings.
3. **Update channel: A.** A signed static manifest on likhon.app, read only when the person taps Check for updates.
4. **Downloads are signed `.exe` files, not zips.** Portable, the installer and self-updates ship as signed executables. **Library packs stay data only:** zip + `manifest.json` + detached Ed25519 signature, with no executable content (owner confirmed).
5. **Offline installer: yes. Online installer: dropped.** The downloads are Portable and the offline Native add-on installer.
