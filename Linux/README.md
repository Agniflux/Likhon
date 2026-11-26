# Likhon for Linux (IBus)

An IBus engine written in C around the Rust typing engine (`Source/crates/likhon-ffi`). Packaged as
`ibus-likhon_<version>_amd64.deb` for Debian and Ubuntu, and `ibus-likhon-<version>-1.fc44.x86_64.rpm` for Fedora.
A pre-release version uses a tilde in the RPM file name: `0.2.1-alpha.1` becomes
`ibus-likhon-0.2.1~alpha.1-1.fc44.x86_64.rpm`.

| File | What |
| --- | --- |
| `ibus-engine/likhon-ibus.c` | The engine: keys in, preedit and committed text out, the suggestion list (IBus lookup table) |
| `ibus-engine/likhon.xml.in` | The IBus component file (the engine's name, language, symbol) |
| `build.sh` | Builds `out/ibus-engine-likhon` (needs a C compiler, pkg-config, `libibus-1.0-dev` on Debian and Ubuntu or `ibus-devel` on Fedora, Rust) |
| `stage-tree.sh` | Stages the files both packages install under `/usr` (the engine, its IBus component file, and the dictionary with its licence) |
| `build-deb.sh` | Builds the `.deb` into `Product/Linux/` (needs `dpkg-dev`) |
| `ibus-likhon.spec.in` | The RPM spec; `build-rpm.sh` fills in the version |
| `build-rpm.sh` | Builds the `.rpm` into `Product/Linux/` (needs `rpm-build` and `redhat-rpm-config`) |
| `write-sums.sh` | Writes `Product/Linux/SHA256SUMS.txt` for the packages in the folder |
| `tests/headless-typing.sh` | Types through a real `ibus-daemon` into a GTK box on a virtual display |

## Using it

After `sudo dpkg -i ibus-likhon_*.deb` (Debian and Ubuntu) or `sudo dnf install ./ibus-likhon-*.rpm` (Fedora):
Settings > Keyboard > Input Sources > + > Bangla > Likhon (GNOME), or `ibus-setup` on other desktops. If it does not
show, `ibus restart` or log out and in.

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

Fedora 44, IBus 1.5.34: the source build, the installed `.rpm` and the headless typing test pass.

## Not done or not checked

- Wayland, KDE, Qt apps, Firefox and Chrome, and a real look at the candidate window.
- A settings window; layouts other than Probhat and Jatiya (the rest are library items on Windows).
- Alt+Backspace to delete a word, the English-word candidate, and next-word predictions.
- Number keys are wired (Settings, `typing.number_pick`) but only unit-tested, not typed through IBus.
- arm64 packages and signing; the RPM is unsigned and no dnf repository exists yet.
