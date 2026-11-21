# Installer

The offline Windows installer (milestone M8). The plan is in `Desktop/CLAUDE.md` section 9.

- `likhon.iss`: the Inno Setup 6 script.
- `build-installer.ps1`: builds `Desktop/Product/Setup/Likhon-Setup-<version>.exe` and `SHA256SUMS.txt` from a built `likhon.exe`.
- `license/`: the agreement text. It must be written by a lawyer. Until then `license-placeholder.txt` is a clearly marked placeholder, and no release may ship with it. The third-party licenses link shows the generated notices (`Desktop/Source/settings-ui/dist/licenses.txt`); see `LICENSING.md` at the root.
- Built installers go to `Desktop/Product/Setup/`, not here.

## Tool decision (2026-11-21, proposal, owner to confirm): Inno Setup 6

| Need | Inno Setup | NSIS | WiX |
| --- | --- | --- | --- |
| Just me or everyone, chosen at run time | Built in (`PrivilegesRequiredOverridesAllowed=dialog`) | Hand-written | MSI per-user/per-machine is awkward |
| Custom pages and Bangla text | Custom messages and a Pascal `[Code]` section | Possible, more manual | Heavy |
| One small signed .exe, no extra runtime | Yes | Yes | Needs a bootstrapper for an .exe |
| Effort and readability | One readable script | Scripting language | XML and MSI concepts |

Inno Setup is free, small and the quickest to keep correct. NSIS would also work; WiX is more than this needs.

## What it does

- Languages: English and Bangla for Likhon's own texts (`[CustomMessages]`, drafts marked needs review). Inno has no official Bangla file, so the wizard's standard buttons stay English. A native-speaker pass is needed before release.
- "Just me" needs no administrator permission; "Everyone on this PC" asks once.
- Tasks: Windows keyboard list (Win+Space, **everyone only**, runs `likhon.exe --keyboard-add`), Start with Windows (same `Run` value name the app uses), nothing else.
- Offline: downloads nothing. If WebView2 is missing it shows a plain note (typing never needs it).
- Uninstall: runs `likhon.exe --keyboard-remove` first, removes its Run entry, asks whether to keep personal words (silent uninstall keeps them). Copies used as Portable are never touched.

## Look (2026-11-21, Inno Setup 7)

The wizard follows the Likhon design system (light): accent sidebar with the feather mark and a five-step list (Welcome, License, Options, Installing, Finish), the page on `bg`, a pill progress bar, and a banner cropped from the splash artwork on the Welcome page. It uses Inno 7's built-in `windows11` custom style for the controls; the sidebar, step markers and progress bar are drawn in `[Code]`. Hind Siliguri is loaded privately for this window only (not installed). This replaces the v2 section 13 step bar; the screens and their order are unchanged.

- `art/make-art.mjs` makes `mark-*.bmp` (node), `art/make-splash.ps1` makes `splash-*.bmp` (PowerShell). The `.bmp` files are committed so a build needs neither.
- Default install is **C:\Program Files\Likhon** (`PrivilegesRequired=admin` with the install-mode dialog): the person is asked "all users or just me" first, and Windows asks for permission only if "all users" is chosen; "just me" installs to `%LOCALAPPDATA%\Programs\Likhon` with no permission prompt.
- Size: the installer carries two Hind Siliguri fonts and the art (about 1.5 MB uncompressed); record the compressed delta in `SIZE.md` after the next release build.
- Checked by running it (English and Bangla Welcome, License, Options, Installing, Finish). **Not checked:** uninstall window styling, 125% to 200% DPI, Windows 10, an all-users install, and a clean VM. Bangla texts are drafts needing review.

## Not done yet (decisions for the owner)

- **Choose fonts page** (v2 section 13): fonts come from the library per user and the app embeds its own, so the installer offers none. Say if a bundled font option is still wanted.
- **File associations** for layout and theme files: those are plain `.json` files, so there is no own extension to register. Left out rather than invented.
- **Signing:** build, then SignPath `installer` configuration (`Desktop/Signer/signpath/installer.xml`), then the Ed25519 `.sig` and the manifest entry.
- **Clean-VM tests** (just me, everyone, upgrade from 0.1.3, uninstall) and the installed-copy-as-Portable test.
