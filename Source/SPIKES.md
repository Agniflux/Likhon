# Spike findings

Numbers, not impressions (root CLAUDE.md, "How to work"). Mark anything not yet tested as **unverified**.

## M0: toolchain (2026-10-13)

- Rust 1.99.0 with targets `x86_64-pc-windows-msvc`, `i686-pc-windows-msvc`, `wasm32-unknown-unknown`; Visual Studio 2022 Build Tools with the C++ workload. All three targets build the workspace (`likhon-engine` checked for wasm32 and i686).
- `cargo` is installed in the user profile (`%USERPROFILE%\.cargo\bin`). Open a new terminal after installing so it is on the PATH.
- Safety: `scripts/kill-likhon.ps1` stops every `likhon.exe`. Keep another keyboard layout installed before M2.

## M1: engine base (2026-10-13)

- Phonetic: `okkhor` 0.10.0 (MIT, std only), with its `editor` feature for per-keystroke edits (backspaces + output). Independent implementation of the Avro-style rules; Avro's MPL source is not used.
- `riti` (MPL-2.0) is not on crates.io; not evaluated yet.
- Tests: `ami` types আমি; keystroke-by-keystroke typing matches whole-word conversion for the design's samples. The owner's corpus (`tests/corpus/`) is still needed for real coverage.

## M2: Spike A (Portable hook typing), 2026-10-13

Built: `Desktop/Product/Portable/likhon.exe`. Starts in English; F12 toggles Bangla/English; Ctrl+Alt+F12 exits; tray tooltip shows the mode, right-click > Exit.

| Measure | Result |
| --- | --- |
| Exe size | 186,880 bytes |
| Idle memory (3 s after start) | 8.9 MB working set, 1.5 MB private |
| Second launch | exits at once (named mutex); M4 will make it open the top bar |
| Keystroke handling time in the hook (engine + SendInput), owner test 2026-10-13 | 18 converted keys: median 152 µs, p95 189 µs, p99 and max 279 µs. Small sample; repeat over 1,000 keys. |
| Watchdog | 3 hook reinstalls in that session (idle periods over 10 s; reinstalling is harmless) |
| Typing by hand | Owner ran it and converted keys (timing file). Confirm the exact output in Notepad. |
| Exe size with icons, tray menu, Settings preview | 316,928 bytes |

Design choices to revisit:
- The hook callback runs the engine and SendInput itself (the brief prefers an engine thread). First numbers (median 152 µs) are far below the hook timeout, so this is fine for now; revisit with a bigger sample.
- Input injected by other programs is passed through untouched (prevents loops; also means AutoHotkey or remote-control tools are not converted).
- Keys map as on a US keyboard; Backspace ends the word instead of editing it (okkhor editor behaviour).
- Elevated (administrator) windows are not reachable from a normal process (UIPI): expected, not supported.
- Watchdog: reinstalls both hooks after 10 s without input events; reinstall count is in the timing file.

## M3: Spike B (text-service DLL)

Not started. Record: machine-wide registration, results per app (Notepad, Chrome, Firefox, Word, Windows Terminal, a Store app).

**Per-user registration without admin: does not work (tested 2026-10-24, Windows 11 Pro 26100, not elevated).** Owner asked for Portable to add Likhon to Win+Space. Test: the release x64 and x86 `likhon_tsf.dll` copied to `%LOCALAPPDATA%`, the COM class written to `HKCU\Software\Classes\CLSID` (and `WOW6432Node`), the profile and the four categories written by hand to `HKCU\Software\Microsoft\CTF\TIP\{clsid}` (same layout as the HKLM keys `RegisterProfile` writes). Then:
- `Set-WinUserLanguageList` with `0845:{C772DC88-…}{1A047B60-…}` added to bn-BD: saved without error, but the list read back had only `0845:00000445` (Windows drops a profile it does not know).
- `InstallLayoutOrTip` (input.dll) with the same string: returned TRUE, added nothing (language list, Preload and CTF SortOrder unchanged).
- Windows also removed the hand-written `HKCU\…\CTF\TIP` key by itself before the undo ran.
Conclusion: Windows reads text-service profiles only from HKLM, so adding Likhon to the keyboard list needs administrator permission once. Undone afterwards; HKCU and the language list are back as before.

**Machine-wide, from Settings › General (tested 2026-10-24, same PC).** One admin prompt; `%ProgramData%\Likhon\tsf.log` records each step.
- **Loads in:** Word, the new Notepad (Windows 11, packaged), Notepad++, Windows Settings, Explorer, Chrome, Edge and WebView2 hosts (the DLL was found loaded in each). Typing checked by hand in Word, Notepad++ and Notepad ("ami" gives আমি with Phonetic, ঋময with Jatiya, so the layout follows settings.json).
- **Apps already open keep the old DLL** after an update (Windows renames it `.old`, deleted at restart); restart the app to get the new one.
- **Suggestions** (agent's window, tsfbridge.rs): shown in Word and Notepad++; Up/Down, Space and Enter work. Handling the keyboard's WM_COPYDATA in place stalled: the password check (UI Automation) asks the app, which is waiting on the agent; the "hide" after a commit timed out. Fixed by queueing and handling just after (MSG_BRIDGE).
- **Word position:** `ITfContextView::GetTextExt` right after the edit often has no layout yet; measured again in a follow-up async edit, else the agent uses its caret lookup.
- **Two taskbar icons:** a `GUID_LBI_INPUTMODE` button showed beside the keyboard's own icon (Windows already shows the profile icon in the input indicator, in place of বা). Removing `TF_LBI_STYLE_SHOWNINTRAY` or adding `TF_LBI_STYLE_HIDDENSTATUSCONTROL` did not help; the button was removed. The profile icon (the logo) is the one icon.
- **Own suggestion window (2026-10-24):** with no likhon.exe running, a fresh Notepad++ showed the keyboard's list under the word and the taskbar showed one Likhon logo. The second logo seen before came from apps that still had the older DLL loaded (with its language bar button); it goes once they restart.
- **Other keyboards:** this PC also has Alo Keyboard (a TSF keyboard) and Avro (a hook). Win+Space quick presses toggle between the last two keyboards, so test switching was done with `ITfInputProcessorProfileMgr::ActivateProfile(…, TF_IPPMF_FORSESSION)`.

## M4: Spike C (agent and Settings in one exe)

Not started. Record: behavior when WebView2 is missing, second-launch handoff time.

## M5: tray states (2026-10-21)

- Tray icon follows v2 §2: Bangla, English, paused (focused window runs as administrator: its token cannot be queried, or it is elevated and Likhon is not), error (keyboard hook not installed). Light or dark taskbar from `SystemUsesLightTheme`, refreshed on `WM_SETTINGCHANGE` "ImmersiveColorSet". Foreground changes come from `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`, out of context.
- Startup check: the agent ran 4 s and stopped cleanly with `kill-likhon.ps1`. Idle working set was 67.5 MB.
- **Cause found (2026-10-21):** the top bar's Direct2D *hardware* render target loaded the Intel GPU driver into the agent (`igc64.dll` 61 MB, `igd10um64xe.dll` 25 MB mapped). Switched the bar to `D2D1_RENDER_TARGET_TYPE_SOFTWARE` (small window, rare redraws).

| Measure (5 s after start, this PC, Intel graphics) | Hardware target | Software target |
| --- | --- | --- |
| Working set | 65.7 MB | **21.6 MB** |
| Private memory | 42.8 MB | **5.7 MB** |
| Loaded modules | 58 | 46 (no GPU driver) |

  Drawing quality looks the same at this size (check by hand on a 200% display). The suggestion window (M5) should start with a software target too.
- By hand, still to check: the paused icon over an administrator window (Task Manager run as administrator), and both taskbar themes.

## Windows code signing: Desktop/Signer (2026-10-25)

- Let's Encrypt cannot sign Windows files (TLS certificates only, no Code Signing usage). Owner chose **SignPath Foundation** (free for open-source; the publisher shown in Windows is "SignPath Foundation"). Setup and the release flow: `Desktop/Signer/README.md`; workflow `.github/workflows/sign-release.yml`.
- `app/build.rs` embeds both `likhon_tsf.dll` files in `likhon.exe`, so a release takes two signing requests: the DLLs first, then the exe built with the signed DLLs inside.
- Tested on this PC: `verify-signature.ps1` passes an embedded-signed file (node.exe, OpenJS Foundation, certificate expired but timestamped, so still valid), and fails an unsigned file (likhon.exe), a catalog-only signature (notepad.exe) and a wrong publisher (explorer.exe against "SignPath Foundation"). `stage.ps1` staged x64 (1,528,832 bytes) and x86 (1,195,520) DLLs and likhon.exe (5,109,760). `use-signed-tsf.ps1` refused the unsigned DLLs.
- **Not tested:** the workflow and SignPath itself (no GitHub repository and no SignPath project yet).
- **Self-signing** (`Desktop/Signer/self/`, 2026-10-25): tested `new-cert.ps1` (RSA 3072, non-exportable key, CurrentUser\My), its refusal when a certificate exists, `sign.ps1` on a copy of likhon.exe (Authenticode, SHA-256, no timestamp), `verify-signature.ps1 -AllowNoTimestamp` failing as expected while the root is not trusted ("terminated in a root certificate which is not trusted"), and `remove-cert.ps1` leaving the store clean. Later the same day: `trust-cert.ps1` (CurrentUser; it must run in a visible window, because a non-interactive shell fails with "UI is not allowed" before Windows can show the Root store prompt) and `sign-build.ps1` end to end, for a copy built for another site (`-Site`, 5,088,072 bytes, into Portablefx) and the regular copy. All three files verified OK. Run the build script from a plain `powershell -File`: redirecting its output (`2>&1`) in PowerShell 5.1 turns cargo's progress lines into a stop error. **Not tested:** `untrust-cert.ps1`.
