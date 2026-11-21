# Roadmap

Singline is the cross-platform successor of Lyrics Overlay: one code base for Windows, macOS and Linux that works with Spotify, Apple Music, Amazon Music and web players. Until Singline has every feature of the native macOS app in [`apps/macos`](../apps/macos), that app remains available for macOS.

## Principles

| Topic | Decision |
|---|---|
| Technology | Tauri 2 with a Rust core and a Svelte 5 and TypeScript interface |
| License | GPL-3.0-or-later; the native macOS app in `apps/macos` stays under MIT |
| Repository | One project on `main`: Singline at the top level, the native macOS app in `apps/macos` |
| Testing | Linux automatically in CI (virtual display, simulated player and lyrics server); Windows and macOS on real devices with [TESTING.md](TESTING.md) |
| Scope | Every feature of the native macOS app, including those contributed from Linory, on every system, and more; tracked in [PARITY.md](PARITY.md) |
| Design | Black and see-through with white lyrics; layout and motion follow the native macOS app |
| Name | Singline (to sing + line). A trademark search follows before version 1.0. |

## Milestones

### Foundation (done)

- [x] Lyrics core in Rust, with every check of the Swift app
- [x] Reading players through MPRIS, the Windows media controls and AppleScript, and the `singline-watch` diagnostic tool
- [x] LRCLIB search, local cache, own lyrics and per-song timing in SQLite
- [x] The app: overlay, settings, tray menu, shortcut and demo mode
- [x] Linux end-to-end test in CI; Windows and macOS test builds for every commit on `main`
- [x] Measurement C: word timing on LRCLIB ([SPIKES.md](SPIKES.md))
- [ ] Measurements A and B on real Windows and macOS devices

### First alpha

- [x] Own lyrics: LRC import and an editor with a timestamp key
- [x] Hiding the overlay while nothing plays
- [x] A configurable shortcut
- [ ] Installers with automatic updates
- [ ] Fixes from measurements A and B, for example keeping the overlay above full-screen apps on macOS

### Everything the native macOS app can do

Ported from the Swift code contributed from Linory:

- [x] Mini-player with a seek bar, jumping to a line by clicking it, chorus markers and cover art
- [x] Estimated word highlight for line-synced lyrics, splitting long lines, scrolling lines and smooth song changes
- [x] Settings in the glass design, sleep timer and listening statistics
- [x] Glass menu panel with the current song, on macOS and Windows
- [x] Spotify connection with True Shuffle, liking songs and top lists
- [ ] Homebrew

### Every player

- [x] A browser extension for YouTube Music, YouTube, Spotify, Apple Music and Amazon Music on the web
- [ ] The extension in the Chrome Web Store and on Firefox Add-ons; Tidal and Deezer
- [ ] The Amazon Music desktop apps, as far as measurement B allows
- [ ] Any player, with allow and ignore lists
- [ ] Linux: Wayland support through the layer shell, and packages

The first release follows this milestone. Singline then replaces the native macOS app.

### Sing to find

- [x] Recognise a song from a few sung words: Whisper on the device, Genius for the words, LRCLIB for the lyrics
- [x] Start at the sung line and follow the singer
- [ ] Find songs among your own and cached lyrics without Genius, also offline
- [ ] A smaller or larger speech model to choose, and the Metal and GPU builds of whisper.cpp

### More

More lyrics sources and a choice between versions, an editor for word timing, publishing lyrics to LRCLIB, a delay per audio output, translations, themes, accessibility, and a mode for streaming with OBS.

### Version 1.0

Trademark, website, code signing, and packages for winget, Flathub and the AUR.
