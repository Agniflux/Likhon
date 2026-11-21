# Likhon for macOS

An InputMethodKit input method (`Likhon.app`, installed in `/Library/Input Methods`), written in Swift around
the Rust typing engine through its C interface (`Source/crates/likhon-ffi`).

**Status (2026-11-21): written, never built or run.** It was written on Windows from Apple's documentation, so
the first build on a Mac will find things to fix. Linux (`Linux/`) is the tested sibling and uses the
same Rust layer.

| File | What |
| --- | --- |
| `Sources/main.swift` | Starts the input-method server and the suggestion panel |
| `Sources/LikhonInputController.swift` | One controller per text field: keys in, marked text and committed text out |
| `Info.plist.in` | Bundle and input-source declaration (Bengali, "Likhon Phonetic"); the version is filled in |
| `Likhon-Bridging-Header.h` | Gives Swift the C interface (`likhon.h`) |
| `build-mac.sh` | Universal build, ad-hoc signature, `.pkg`, `.dmg` into `Product/Mac/` |
| `INSTALL.txt` | The how-to that goes inside the disk image |

## Build (on a Mac)

```
cd Mac
./build-mac.sh
```

Needs Xcode command line tools and Rust. Output: `Product/Mac/Likhon-<version>-macOS.dmg`.

## Not signed (owner decision 2026-11-21)

No Apple Developer ID, so no notarization: people see a Gatekeeper warning and open the installer with
right-click, Open. The app carries an ad-hoc signature only because Apple Silicon will not run unsigned code.
When the owner enrols, add `codesign --options runtime`, `productsign` and `notarytool` to `build-mac.sh`.

## To check on the first Mac run

- The input source appears under System Settings > Keyboard > Input Sources > Bangla after logging out and in.
- Typing `ami ` gives `আমি ` in TextEdit, Safari, Notes, Terminal and a browser text box.
- The suggestion panel (`IMKCandidates`) shows below the word, Up and Down move the selection (the code handles
  the arrows itself; whether the panel also reacts to them is **unverified**), Space and Enter type the selected row.
- Apps that cannot show marked text (some terminals, Google Docs, Electron apps): Nukta's notes say these need
  the word held back until it ends. Test and decide.
- No icon yet (`tsInputModeMenuIconFileKey` is missing from `Info.plist.in`): the menu shows a generic one.
- A settings window, layouts other than Phonetic, and learning words are not built.
