<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="Reference/design-v2/assets/likhon-mark-blue-dark.svg">
    <img src="Reference/design-v2/assets/likhon-mark-blue.svg" width="88" height="88" alt="The Likhon quill">
  </picture>
</p>

<h1 align="center">লিখন · Likhon</h1>

<p align="center">
  <strong>Free Bangla typing for Windows, macOS and Linux.</strong><br>
  Type <code>ami</code>, get <strong>আমি</strong>. No account, no tracking, no paid tier.
</p>

<p align="center">
  <img alt="Status: alpha" src="https://img.shields.io/badge/status-alpha-8A5200?labelColor=1B2230&style=flat-square">
  <img alt="License: MPL-2.0" src="https://img.shields.io/badge/license-MPL--2.0-2747C4?labelColor=1B2230&style=flat-square">
  <img alt="Windows 10 and 11" src="https://img.shields.io/badge/Windows-10%20%7C%2011-2747C4?labelColor=1B2230&style=flat-square">
  <img alt="macOS 13 or later" src="https://img.shields.io/badge/macOS-13%2B-2747C4?labelColor=1B2230&style=flat-square">
  <img alt="Linux with IBus" src="https://img.shields.io/badge/Linux-IBus-2747C4?labelColor=1B2230&style=flat-square">
</p>

<p align="center">
  <img src="Reference/design-v2/assets/splash.png" width="640" alt="Likhon: a Bangla phonetic keyboard with a suggestion list">
</p>

---

## What it is

Likhon turns English letters into Bangla as you type, in any app. It shows the word you are typing, suggests
spellings from a built-in dictionary, fixes common slips, and learns the words you use, on your own computer only.

Everything in Likhon is **alpha**: it works and we use it, and it still has rough edges.

| System | What you get | State |
| --- | --- | --- |
| **Windows 10 and 11** | The full app: Portable (one file, no install) and an offline installer. Tray, top bar, suggestion window, Settings, library, legacy-font tools. | Alpha |
| **macOS 13 or later** | An input method, one download for Apple Silicon and Intel. Not signed with an Apple Developer ID yet. | Alpha, **not yet tested on a Mac** |
| **Linux (IBus)** | An IBus engine as a `.deb`. Settings are a small file for now. | Alpha |

Typing: phonetic (Avro-style), Probhat and Jatiya fixed layouts, dictionary suggestions, autocorrect, emoji by name,
Bangla digits, and a choice for what `.` and `:` type. More layouts and themes come from the library inside the
Windows app.

## Privacy

- What you type stays on your computer. It is never sent anywhere and never written to a log.
- Learned words are kept in a file you can read and delete, and never learned in password fields.
- Likhon uses the internet only when you ask it to (check for updates, open the library, download the dictionary,
  donate words). No telemetry, no background connections.

## Build from source

```sh
git clone https://github.com/Agniflux/Likhon.git
cd Likhon
```

You need [Rust](https://rustup.rs) (stable). Every command below starts in the repository root.

**Test the typing engine (any system)**

```sh
cd Source
cargo test -p likhon-engine -p likhon-store -p likhon-ffi
```

**Windows** (MSVC toolchain, Node 20 or later)

```powershell
cd Source
cd settings-ui; npm ci; npm run build; cd ..                       # the Settings window (embedded in the exe)
rustup target add i686-pc-windows-msvc
cargo build --release -p likhon-tsf                                 # the Win+Space keyboard, 64-bit
cargo build --release -p likhon-tsf --target i686-pc-windows-msvc   # and 32-bit (likhon.exe carries both)
cargo build --release -p likhon                                     # likhon.exe
powershell -File scripts\make-portable.ps1                          # Portable folder in Product\Portable
powershell -File ..\Installer\build-installer.ps1                  # offline installer (Inno Setup 6 or 7)
```

**Linux** (`build-essential`, `pkg-config`, `libibus-1.0-dev`, `dpkg-dev`)

```sh
cd Linux
./build.sh && ./tests/headless-typing.sh && ./build-deb.sh   # .deb in Product/Linux
```

**macOS** (Xcode command line tools)

```sh
cd Mac
./build-mac.sh                                               # .dmg in Product/Mac
```

The **Build Linux and macOS** workflow (`.github/workflows/build-unix.yml`) runs the Linux and macOS builds on GitHub.
Read `Mac/README.md` first: the macOS code has never run on a real Mac.

## The dictionary

`Source/assets/dict/mpl/words.sqlite` is the Avro Keyboard word list (159,380 words, 737 suffixes, 2,222 autocorrect
entries), MPL-2.0, with its source in `SOURCE.md`. A builder can put a larger `words.full.sqlite` beside it: the build
scripts use that file when it exists, and it is never committed.

## What is in this repository

```
Source/       Rust workspace: the typing engine, the dictionary store, the C interface, the Windows app and keyboard
              (settings-ui is the Settings window), assets, scripts, tests
Linux/        IBus engine (C) and the .deb
Mac/          InputMethodKit app (Swift) and the .dmg
Installer/    the Windows installer (Inno Setup) and its licence text
Reference/    a copy of the design tokens, icon set and artwork the builds read (see Design)
```

## Design

Likhon's look comes from one set of tokens, `Reference/design-tokens.json`: warm paper (`#FBF9F4`) and ink
(`#1B2230`), one blue (`#2747C4`) for what is on or clickable, amber for warnings, a duotone icon set, and Hind
Siliguri for Bangla and Latin text. The apps, the Settings window and the installer read the same tokens.
`Reference/` holds a snapshot of those files so the repository builds on its own.

## Contributing

Issues and pull requests are welcome. Run `cargo test -p likhon-engine -p likhon-store -p likhon-ffi` before you send a
change to the typing engine: the tests are the typing rules. New text for the apps goes through the text library in
`Source/shared/text/` (English first; Bangla is reviewed by a native speaker).

## License and credits

Likhon's own code is under the **[Mozilla Public License 2.0](LICENSE)**.

Likhon stands on other people's work, each under its own licence:

- **Avro Keyboard** (OmicronLab, MPL-2.0): the word list, autocorrect list, fixed layouts and themes in
  `Source/assets/`, each folder with its own `LICENSE` and `SOURCE.md`.
- **okkhor** (MIT): the phonetic rules. **Khipro** (MIT): the Khipro input method.
- **Hind Siliguri** (Indian Type Foundry, SIL OFL 1.1): the typeface.
- Rust crates, mostly MIT or Apache-2.0: the full list is generated into the app's *About* page
  (`Source/assets/licenses/`).

Product and brand names belong to their owners. Likhon is not made or endorsed by them.

Likhon is a brand of **Agniflux Enterprise**, Dhaka.
