# Likhon (লিখন)

<img src="Reference/design-v2/assets/likhon-icon.svg" width="120" alt="Likhon">

**Version 0.2.1** · alpha · [MPL-2.0](LICENSE)

**Supported systems**

- **Windows** 10 and 11 (64-bit)
- **macOS** 13 (Ventura) or later, on Apple Silicon and Intel
- **Linux** with IBus 1.5 or later, 64-bit Intel or AMD (the `.deb` needs glibc 2.31 or later: Ubuntu 20.04 or Debian 11
  and newer; tested on Ubuntu 24.04)

Free Bangla typing. You write English letters (`ami`) and Likhon gives you Bangla (আমি), shows spelling suggestions as
you type, and learns the words you use. There is no account, no tracking and no paid tier, and the same Rust typing
engine runs on all three systems and in the browser tools on the website.

On Windows you get the full app, with a tray icon, a top bar, a suggestion window and a Settings window, as Portable
(one file, nothing to install) or an installer. On Mac it is an input method, one download for Apple Silicon and Intel.
On Linux it is an IBus engine as a `.deb` package.

Everything in Likhon is **alpha**: it works and we use it, and it still has rough edges. What is not done yet is in
[ENHANCEMENTS.md](ENHANCEMENTS.md). The Mac version has never been run on a real Mac.

## Features

- **Phonetic typing**: Avro-style rules. `ami` gives আমি, `bangladesh` gives বাংলাদেশ, `.` gives ।, and `..` gives
  a full stop. The word you are typing stays underlined until it ends.
- **Suggestions**: a list under the word, from a built-in dictionary of about 160,000 words, with common slips fixed
  as you type (`account` becomes অ্যাকাউন্ট). Easily confused letters (ি and ী, স, শ and ষ) are offered as spellings.
  `Space` types the selected row; `Up`, `Down` and `Enter` choose another.
- **Fixed layouts**: Probhat and Jatiya, typed by key position. On Windows, more layouts and themes (UniJoy, Khipro,
  Munir, Avro Easy) are added from the library with one click.
- **Your words**: finished words are remembered on your computer so they come up first next time, and are never
  learned in password fields. On Windows you can see, export and delete them, and Likhon can predict the next word.
- **Digits and punctuation**: Bangla or Latin digits, and a choice for what `:` types (a visarga ঃ or a plain colon).
- **Old fonts and encodings** (Windows): convert between Unicode and Bijoy or Proshika in both directions, type straight
  into an old font, or convert text you already have with one shortcut.
- **Tools** (Windows): the converter, a cleaner for pasted text, Taka in words, Bangla numbers and dates, "How do I type
  this word?", a symbol palette, an on-screen keyboard and a printable layout chart. The same tools run in the browser
  at likhon.agniflux.com with no text sent anywhere.
- **Emoji by name** (Windows): `:hashi:` or `:smile:` gives 😀.
- **Windows keyboard list** (Windows): add Likhon to `Win` + `Space` from any copy, so it works in every app and
  shows its own suggestions.
- **Themes and languages** (Windows): Default, Paper and Green, each light and dark, and an English and Bangla
  interface that switches at once.
- **Offline by default**: Likhon uses the internet only when you ask it to (check for updates, open the library,
  download the dictionary, donate words). No telemetry and no background connections.

## Getting started

### Windows

1. Download **Portable** (`likhon.exe`, one file) or **Full** (the installer) from likhon.agniflux.com.
2. Portable: put `likhon.exe` anywhere, a folder or a USB stick, and run it. It keeps its settings and your words
   beside it. Full: run the installer and choose *just me* or *everyone on this PC*. The installer is offline and
   downloads nothing. *Everyone* asks for administrator permission once; Likhon's own typing never needs it.
3. Windows may show "Windows protected your PC" for a new app that few people have run. Click **More info**, then
   **Run anyway**.

Then `F12` switches between Bangla and English. The tray icon shows which is on, and double-clicking it opens the top bar.
`Up` and `Down` move in the suggestion list, `Space` or `Enter` type the selected word, `Esc` closes the list. To type
from the Windows keyboard list, open Settings › General and add the Likhon keyboard; Windows asks for administrator
permission once, because it only reads keyboards from a system folder. Likhon cannot type into windows running as
administrator; that is a Windows rule. Tools are under `Ctrl` + `Alt` + `T`.

### Mac

1. Open the `.dmg` and then **Install Likhon.pkg**. This build is not signed with an Apple Developer ID yet, so macOS
   may say it cannot be opened. Control-click the file, choose **Open**, then **Open** again. Or go to System Settings →
   Privacy & Security and choose **Open Anyway**.
2. Type your Mac password when asked, then log out and back in.
3. System Settings → Keyboard → Input Sources → **Edit…** → **+** → Bangla → **Likhon**.

Switch with the Input menu in the menu bar, or `Control` + `Space`. Apps that cannot show a word being typed (some
terminals, Google Docs, Electron apps) may need more work.

### Linux

```sh
sudo dpkg -i ibus-likhon_*.deb
```

Then log out and back in, or run `ibus restart`. Add **Likhon** under Settings → Keyboard → Input Sources → **+** →
Bangla (GNOME), or run `ibus-setup` on other desktops. It needs IBus; on Debian and Ubuntu, `im-config -n ibus` makes
IBus your input method. Switch with `Super` + `Space`.

The word you are typing is underlined. `Up` and `Down` move in the suggestion list, `Space` types the selected word,
`Enter` types it without a space, `Esc` drops the word and `Backspace` takes one letter off it. We have tested it on
Ubuntu 24.04 with IBus 1.5; Wayland, KDE, Qt apps, Firefox and Chrome are still to be checked.

## Settings

On **Windows**, everything is in the Settings window (tray icon → Open settings). Layouts, themes, the dictionary
and library items are added from there, and only when you click.

On **Mac and Linux** there is no settings window yet. Linux reads `~/.config/likhon/settings.json`, with the same
names as the Windows app, and picks up a change the next time a text box gets focus:

```json
{
  "layout": "phonetic",
  "typing.suggestions": true,
  "typing.autocorrect": true,
  "typing.number_pick": false,
  "typing.learn": true,
  "typing.bangla_digits": false,
  "typing.dari": true,
  "typing.visarga": true
}
```

`layout` is `phonetic`, `probhat` or `jatiya`. Suggestions and autocorrect work with `phonetic` only. With
`typing.number_pick` on, `1` to `9` take that row of the suggestion list while you type a word. Learned words are kept
in `~/.local/share/likhon/user.sqlite`; delete the file to forget them.

## Downloads

- **Windows**: `likhon.exe` (Portable) and `Likhon-Setup-<version>.exe` (the installer) from likhon.agniflux.com. Both carry a
  Likhon signature. A newer file replaces the old one and keeps your words.
- **Linux**: `ibus-likhon_<version>_amd64.deb`.
- **Mac**: `Likhon-<version>-macOS.dmg`, one file for Apple Silicon and Intel.

The Linux and Mac files are built by the **Build Linux and macOS** workflow (`.github/workflows/build-unix.yml`) when you
push a tag that starts with `v` (for example `git tag v0.2.1-alpha.1 && git push origin v0.2.1-alpha.1`). Open the run on
the Actions tab and download `likhon-linux` or `likhon-macos` from its Artifacts.

## Building from source

You need [Rust](https://rustup.rs) (stable). Every command starts in the repository root.

```sh
git clone https://github.com/Agniflux/Likhon.git
cd Likhon/Source
cargo test -p likhon-engine -p likhon-store -p likhon-ffi     # the typing rules, on any system
```

**Windows** (MSVC toolchain, Node 20 or later):

```powershell
cd Source
cd settings-ui; npm ci; npm run build; cd ..
rustup target add i686-pc-windows-msvc
cargo build --release -p likhon-tsf                                  # the Win+Space keyboard, 64-bit
cargo build --release -p likhon-tsf --target i686-pc-windows-msvc    # and 32-bit; likhon.exe carries both
cargo build --release -p likhon                                      # likhon.exe
powershell -File scripts\make-portable.ps1                           # Portable folder in Product\Portable
powershell -File ..\Installer\build-installer.ps1                    # the installer (Inno Setup 6 or 7)
```

The app talks to `https://likhon.agniflux.com` (updates, library, dictionary), and only when you click. To build a copy
that talks to another site, for example a local one, set `LIKHON_SITE` first (`$env:LIKHON_SITE = "https://example.org"`)
or run `scripts\make-portable.ps1 -Site https://example.org -Sub other`.

**Linux** (`build-essential`, `pkg-config`, `libibus-1.0-dev`, `dpkg-dev`):

```sh
cd Linux
./build.sh                        # out/ibus-engine-likhon
./tests/headless-typing.sh        # types through a real ibus-daemon on a virtual display
./build-deb.sh                    # Product/Linux/ibus-likhon_<version>_amd64.deb
```

**Mac** (Xcode command line tools):

```sh
cd Mac
./build-mac.sh                    # Product/Mac/Likhon-<version>-macOS.dmg
```

The Mac code was written without a Mac to run it on, so expect the first build to need fixes. Read `Mac/README.md`.

## Design

Likhon's look comes from one set of tokens, `Reference/design-tokens.json`, used by the Settings window, the native
windows and the installer alike: warm paper and ink, one blue for what is on or clickable, amber for warnings, and a
duotone icon set. **Hind Siliguri** is the typeface for Bangla and Latin text.

- **Light**: accent blue `#2747C4` (hover `#1F3AA6`, soft ground `#E6EBFB`) on `#FBF9F4` paper with `#FFFFFF` cards,
  `#1B2230` text, `#5B6474` secondary text and `#E3DED3` outlines.
- **Dark**: accent `#8FA6FF` on `#151922`, with `#1C212C` cards, `#E8EBF2` text and `#2E3544` outlines.
- Warnings are amber (`#8A5200`, `#F2B24A` in dark), errors red (`#C2362B`) and success green (`#1F7A4D`), each with a
  soft ground for pills and notices.

`Reference/` holds a snapshot of the tokens, the icon set and the artwork the builds read, so this repository builds
on its own.

## Project structure

```
Source/
  Cargo.toml                 the Rust workspace
  crates/
    likhon-engine/           the typing engine, pure logic with no I/O
      src/lib.rs               phonetic typing, one key at a time
      src/fixed.rs             fixed layouts (Probhat, Jatiya, ...) from JSON files
      src/proshika.rs          Bijoy and Proshika conversion tables
      src/mit/                 Khipro, from the Khipro team's MIT input-method file
      src/emoji/, tools.rs     emoji by name; Taka, numbers and dates, the cleaner
    likhon-store/            the dictionary (read-only SQLite), your words, and the suggestion ranking
    likhon-ffi/              a C interface for Mac and Linux: one session per text field (include/likhon.h)
    likhon-wasm/             the engine as WebAssembly, for the website's browser tools
  app/                       likhon.exe (Windows): agent.rs (keyboard hook), topbar.rs, suggest.rs, osk.rs,
                             settings_app.rs (the Settings window host), online.rs and net.rs (the only network code)
  tsf/                       likhon_tsf.dll, the Win+Space keyboard
  settings-ui/               the Settings window (TypeScript, built to dist/ and embedded in the exe)
  assets/                    fonts, icons, layouts, themes, the dictionary and licence texts
  shared/                    the text library (English source, Bangla marked for review), schemas, icon export
  scripts/                   build, conversion and measuring scripts
Linux/                       the IBus engine in C and the .deb
Mac/                         the InputMethodKit app in Swift and the .dmg
Installer/                   the Windows installer (Inno Setup) and its licence text
Reference/                   the design snapshot (see Design)
```

Notes for contributors:

- **The engine is the product.** Every typing rule lives in `likhon-engine` and is covered by tests; the Windows hook,
  the Windows keyboard, the Mac input method and the Linux engine only pass keys in and show what comes back. Run the
  tests before you send a change to it.
- **Words, not keystrokes, are what is learned.** Nothing you type is stored or logged. A word is learned only when it
  ends, only if it is Bangla letters, and never in a password field.
- **All text goes through the text library** in `Source/shared/text/`, English first. Bangla is reviewed by a native
  speaker before it is called reviewed.
- **Mac and Linux share one Rust layer** (`likhon-ffi`), which holds the word being typed and returns the text to
  insert and the text to underline. The shells in `Mac/` and `Linux/` are thin on purpose.

## The dictionary

`Source/assets/dict/mpl/words.sqlite` is the Avro Keyboard word list: 159,380 words, 737 suffixes and 2,222 autocorrect
entries, under the MPL-2.0, with its source in `SOURCE.md`. A builder can put a larger `words.full.sqlite` beside it;
the build scripts use that file when it exists, and it is never committed.

## Privacy

What you type stays on your computer. It is not sent anywhere and not written to a log. Likhon uses the internet only
when you ask it to, and it does nothing in the background.

## Credits

Likhon is developed by [Agniflux Enterprise](https://www.agniflux.com), Dhaka. Likhon's own code is under the
[Mozilla Public License 2.0](LICENSE).

It stands on other people's work, each under its own license: **Avro Keyboard** (OmicronLab, MPL-2.0) for the word
list, autocorrect list, layouts and themes in `Source/assets/`, each folder with its own `LICENSE` and `SOURCE.md`;
**okkhor** (MIT) for the phonetic rules; **Khipro** (MIT); **Hind Siliguri** (Indian Type Foundry, SIL OFL 1.1) for the
typeface; and Rust crates, mostly MIT or Apache-2.0, listed in the app's *About* page (`Source/assets/licenses/`).
Product and brand names belong to their owners, and Likhon is not made or endorsed by them.
