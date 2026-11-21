# Likhon for Linux (IBus)

An IBus engine written in C around the Rust typing engine (`Source/crates/likhon-ffi`). Packaged as
`ibus-likhon_<version>_amd64.deb`.

| File | What |
| --- | --- |
| `ibus-engine/likhon-ibus.c` | The engine: keys in, preedit and committed text out, the suggestion list (IBus lookup table) |
| `ibus-engine/likhon.xml.in` | The IBus component file (the engine's name, language, symbol) |
| `build.sh` | Builds `out/ibus-engine-likhon` (needs a C compiler, pkg-config, `libibus-1.0-dev`, Rust) |
| `build-deb.sh` | Builds the `.deb` into `Product/Linux/` (needs `dpkg-dev`) |
| `tests/headless-typing.sh` | Types through a real `ibus-daemon` into a GTK box on a virtual display |

## Using it

After `sudo dpkg -i ibus-likhon_*.deb`: Settings > Keyboard > Input Sources > + > Bangla > Likhon (GNOME), or
`ibus-setup` on other desktops. If it does not show, `ibus restart` or log out and in.

## Settings

There is no settings window yet. The engine reads `~/.config/likhon/settings.json`, the same names as the Windows
app's `settings.json`, and re-reads it when a text box gets focus if the file changed. Missing keys keep the defaults.

```json
{
  "layout": "phonetic",
  "typing.suggestions": true,
  "typing.autocorrect": true,
  "typing.preselect": "dict",
  "typing.number_pick": false,
  "typing.learn": true,
  "typing.bangla_digits": false,
  "typing.dari": true,
  "typing.visarga": true
}
```

`layout` is `phonetic`, `probhat` or `jatiya`. Suggestions and autocorrect work with `phonetic` only.

## What it keeps

- Your learned words and "my words": `~/.local/share/likhon/user.sqlite` (this user only). Words are learned only
  when a word ends, only Bangla letters, and **never in password or PIN fields** (IBus tells the engine which
  fields those are). Turn it off with `"typing.learn": false`; delete the file to forget everything.
- Nothing else is stored, and nothing is sent anywhere.

## Tested

Ubuntu 24.04 (WSL), IBus 1.5.29, a GTK 3 text box on a virtual X display: the phonetic sentence, dictionary
autocorrect, Down then Space picking another row, learning, the settings file (colon, Bangla digits, Probhat) and
a password box. The installed `.deb` passes the same run.

## Not done or not checked

- Wayland, KDE, Qt apps, Firefox and Chrome, and a real look at the candidate window.
- A settings window; layouts other than Probhat and Jatiya (the rest are library items on Windows).
- Alt+Backspace to delete a word, the English-word candidate, and next-word predictions.
- Number keys are wired (Settings, `typing.number_pick`) but only unit-tested, not typed through IBus.
- arm64 packages, CI (`.github/workflows/build-unix.yml` is ready for when the repo is on GitHub), signing.
