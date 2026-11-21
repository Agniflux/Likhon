# Enhancements

What Likhon does not do yet, and what we would like to add. Everything in Likhon is alpha, so this list is long and
honest. Nothing here is promised for a date.

## Everywhere

- A native-speaker review of every Bangla text. Today all of it is marked *needs review*.
- Typing: a typed English word as one of the suggestion rows (for mixed Bangla and English), `Alt` + `Backspace` to
  delete a whole word, and next-word suggestions on Mac and Linux (Windows has them).
- A choice of how the suggestion list is laid out (above or below the word, a column or a row).
- A Bangla test corpus for the engine, so the typing rules are checked against real sentences.

## Windows

- **Signing.** The files carry a Likhon signature, but are not yet signed with a code-signing certificate Windows
  recognises, so SmartScreen warns. A free certificate for open-source projects is the plan.
- **Self-update.** Settings › Updates tells you when a newer version exists, and you download it yourself. The helper
  that swaps the exe in place is not built.
- **ARM64.** Only 64-bit and 32-bit x86 builds exist.
- **Per-user keyboard.** Adding Likhon to the Win+Space list needs administrator permission once. We tried a per-user
  registration and Windows refuses to read it.
- More legacy encodings (Lekhani, Shahid Lipi) and a layout and theme designer.

## Mac

- **A first run on a real Mac.** The code has been written and has never been built or run on one. Expect fixes.
- Signing with an Apple Developer ID and notarisation, so macOS stops warning.
- A settings window, Probhat and Jatiya, learned words, and an icon in the input menu.
- Apps that cannot show a word being typed (some terminals, Google Docs, Electron apps).
- A Homebrew cask, once the build is signed.

## Linux

- A settings window. Settings are a file for now.
- More testing: Wayland, KDE, Qt apps, Firefox and Chrome, and a real look at the candidate window.
- A package for ARM64 and a signed apt repository, so `apt install` works.
- Fcitx support, for people who do not use IBus.
