# Desktop source

The Rust workspace behind Likhon. The repository README has the build steps for each system.

| Folder | What |
| --- | --- |
| `crates/likhon-engine` | The typing engine: phonetic rules, fixed layouts, converters, number and Taka tools. Pure logic, no I/O. |
| `crates/likhon-store` | The dictionary (read-only SQLite), the person's words, and the shared suggestion ranking. |
| `crates/likhon-ffi` | A C interface for the macOS and Linux input methods: one session per text field, preedit and committed text out. |
| `crates/likhon-wasm` | The engine built to WebAssembly for browser tools (built by `scripts/build-wasm.ps1`). |
| `app/` | `likhon.exe` (Windows): tray, top bar, suggestion window, Settings host. |
| `tsf/` | `likhon_tsf.dll` (Windows): the Win+Space keyboard. |
| `settings-ui/` | The Settings window (TypeScript, built to `dist/` and embedded in the exe). |
| `assets/` | Fonts, icons, themes, layouts, the dictionary, licence texts. Each third-party folder has its own `LICENSE` and `SOURCE.md`. |
| `shared/` | The text library (English source, Bangla marked for review), schemas, icon export. |
| `scripts/` | Build, conversion and measuring scripts. |

`SIZE.md` logs the exe size, `SPIKES.md` the technical findings and measurements, `PROPOSALS.md` the design decisions.

The dictionary in `assets/dict/mpl/words.sqlite` is the Avro-derived word list only. A builder can place a larger
`words.full.sqlite` beside it; the build scripts use that file when it exists.
