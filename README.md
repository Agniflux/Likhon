# Likhon

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="Reference/design-v2/assets/likhon-mark-blue-dark.svg">
  <img src="Reference/design-v2/assets/likhon-mark-blue.svg" width="96" alt="The Likhon quill">
</picture>

Type Bangla in any app on your computer. You write English letters (`ami`) and Likhon gives you Bangla (আমি), shows
spelling suggestions as you go, and learns the words you use. It is free, it has no account and no tracking, and
everything in it is **alpha**: it works and we use it, and it still has rough edges.

- **Windows** 10 and 11: the full app, with a tray icon, a top bar, a suggestion window and a Settings window.
  Portable (one file, nothing to install) or an installer. The code is in `Source/app`, `Source/tsf` and `Installer`.
- **Mac** (macOS 13 or later): an input method, one download for Apple Silicon and Intel. The code is in `Mac`.
- **Linux**: an IBus engine as a `.deb` package. The code is in `Linux`.

All three use the same typing engine, written in Rust (`Source/crates`).

What is not done yet is in [ENHANCEMENTS.md](ENHANCEMENTS.md). The Mac version has never been run on a real Mac.

## Typing

Likhon types in the way you choose:

- **Phonetic**: Avro-style rules. `ami` gives আমি, `bangladesh` gives বাংলাদেশ, `.` gives ।, and `..` gives a full stop.
- **Fixed layouts**: Probhat and Jatiya, typed by key position. The Windows app can add more from its library.
- **Suggestions**: a list under the word you are typing, from a built-in dictionary of about 160,000 words. A spelling
  you probably meant is already selected (`account` becomes অ্যাকাউন্ট), and `Space` types the selected row.
- **Digits and punctuation**: Bangla or Latin digits, and a choice for what `:` types (a visarga ঃ or a plain colon).
- **Your words**: finished words are remembered on your computer so they come up first next time. They are never
  learned in password fields.

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

`layout` is `phonetic`, `probhat` or `jatiya`. Suggestions and autocorrect work with `phonetic` only.
With `typing.number_pick` on, `1` to `9` take that row of the suggestion list while you type a word.

## Windows app

### Install

1. Download **Portable** (`likhon.exe`, one file) or **Full** (the installer) from likhon.app.
2. Portable: put `likhon.exe` anywhere, a folder or a USB stick, and run it. It keeps its settings and your words
   beside it. Full: run the installer and choose *just me* or *everyone on this PC*.
3. Windows may show "Windows protected your PC" for a new app that few people have run. Click **More info**, then
   **Run anyway**.

The installer is offline and downloads nothing. *Everyone* asks for administrator permission once. Likhon's own typing
never needs it.

### Using it

- `F12` switches between Bangla and English. The tray icon shows which is on, and double-clicking it opens the top bar.
- `Up` and `Down` move in the suggestion list, `Space` or `Enter` type the selected word, `Esc` closes the list.
- To type in Bangla from the Windows keyboard list (`Win` + `Space`), open Settings › General and add the Likhon
  keyboard. Windows asks for administrator permission once, because it only reads keyboards from a system folder.
  That keyboard shows its own suggestions.
- Likhon cannot type into windows running as administrator. That is a Windows rule.
- The converter, the number and Taka tools and the symbol palette are under `Ctrl` + `Alt` + `T`.

### Building it (developers)

You need Rust (MSVC toolchain) and Node 20 or later.

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

## Mac app

### Install

1. Open the `.dmg` and then **Install Likhon.pkg**. This build is not signed with an Apple Developer ID yet, so macOS
   may say it cannot be opened. Control-click the file, choose **Open**, then **Open** again. Or go to System Settings →
   Privacy & Security and choose **Open Anyway**.
2. Type your Mac password when asked, then log out and back in.
3. System Settings → Keyboard → Input Sources → **Edit…** → **+** → Bangla → **Likhon**.

### Using it

Switch with the Input menu in the menu bar, or `Control` + `Space`. Typing, suggestions and the keys are the same as on
Linux. Apps that cannot show a word being typed (some terminals, Google Docs, Electron apps) may need more work.

### Building it (developers)

You need a Mac with the Xcode command line tools and Rust.

```sh
cd Mac
./build-mac.sh        # Product/Mac/Likhon-<version>-macOS.dmg
```

The code was written without a Mac to run it on, so expect the first build to need fixes. Read `Mac/README.md`.

## Linux

### Install

```sh
sudo dpkg -i ibus-likhon_*.deb
```

Then log out and back in, or run `ibus restart`. Add **Likhon** under Settings → Keyboard → Input Sources → **+** →
Bangla (GNOME), or run `ibus-setup` on other desktops. It needs IBus; on Debian and Ubuntu, `im-config -n ibus` makes
IBus your input method. Switch with `Super` + `Space`.

### Using it

The word you are typing is underlined. `Up` and `Down` move in the suggestion list, `Space` types the selected word,
`Enter` types it without a space, `Esc` drops the word and `Backspace` takes one letter off it. Learned words are kept in
`~/.local/share/likhon/user.sqlite`. Delete the file to forget them.

We have tested it on Ubuntu 24.04 with IBus 1.5. Wayland, KDE, Qt apps, Firefox and Chrome are still to be checked.

### Building it (developers)

You need `build-essential`, `pkg-config`, `libibus-1.0-dev`, `dpkg-dev` and Rust.

```sh
cd Linux
./build.sh                        # out/ibus-engine-likhon
./tests/headless-typing.sh        # types through a real ibus-daemon on a virtual display
./build-deb.sh                    # Product/Linux/ibus-likhon_<version>_amd64.deb
```

## Testing the engine

On any system:

```sh
cd Source
cargo test -p likhon-engine -p likhon-store -p likhon-ffi
```

The **Build Linux and macOS** workflow (`.github/workflows/build-unix.yml`) runs the Linux and macOS builds on GitHub
when you push a tag that starts with `v`.

## The dictionary

`Source/assets/dict/mpl/words.sqlite` is the Avro Keyboard word list: 159,380 words, 737 suffixes and 2,222 autocorrect
entries, under the MPL-2.0, with its source in `SOURCE.md`. A builder can put a larger `words.full.sqlite` beside it;
the build scripts use that file when it exists, and it is never committed.

## Privacy

What you type stays on your computer. It is not sent anywhere and not written to a log. Likhon uses the internet only
when you ask it to (check for updates, open the library, download the dictionary, donate words), and it does nothing in
the background.

## License and credits

Likhon's own code is under the [Mozilla Public License 2.0](LICENSE).

It stands on other people's work, each under its own license: **Avro Keyboard** (OmicronLab, MPL-2.0) for the word
list, autocorrect list, layouts and themes in `Source/assets/`, each folder with its own `LICENSE` and `SOURCE.md`;
**okkhor** (MIT) for the phonetic rules; **Khipro** (MIT); **Hind Siliguri** (Indian Type Foundry, SIL OFL 1.1) for the
typeface; and Rust crates, mostly MIT or Apache-2.0, listed in the app's *About* page (`Source/assets/licenses/`).
Product and brand names belong to their owners, and Likhon is not made or endorsed by them.

The design tokens, icon set and artwork the builds read are copied into `Reference/`.

Likhon is a brand of **Agniflux Enterprise**, Dhaka.
