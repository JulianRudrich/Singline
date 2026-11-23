# Architecture

This page explains how Singline is put together: which part does what, how a line of lyrics gets from the player to the screen, and where data lives. It is meant for anyone who wants to change the code. The module comments in each file go into the details.

## The parts

Singline is a [Tauri 2](https://tauri.app) app. A Rust process reads the players, finds the lyrics and opens the windows; each window shows a Svelte page.

| Path | What it does |
|---|---|
| [`crates/lyrics-core`](../crates/lyrics-core) | Lyrics logic without any input or output: LRC and Lyricsfile parsing with word timing, matching LRCLIB results to a song, the playback clock, choosing a player, the chorus, plain lyrics, the listening statistics, and lining up sung words with lyrics. Every function is deterministic and covered by unit tests. |
| [`crates/media-session`](../crates/media-session) | Reads and controls the players: MPRIS on Linux, the system media controls (SMTC) on Windows, AppleScript on macOS, the browser bridge for the extension, and the microphone player for Sing to find. Also `singline-watch`, the diagnostic tool. |
| [`crates/lyrics-providers`](../crates/lyrics-providers) | Finds lyrics: your own lyrics, then the cache, then [LRCLIB](https://lrclib.net). Holds the SQLite store, and the Genius search that turns sung words into candidate songs. |
| [`crates/spotify-connect`](../crates/spotify-connect) | The optional Spotify connection: PKCE login with your own Spotify app, the system credential store, the Web API calls and the rules of True Shuffle. |
| [`crates/transcription`](../crates/transcription) | Sing to find on the device: microphone capture, voice detection, the Whisper model download and transcription with whisper.cpp. |
| [`apps/desktop/src-tauri`](../apps/desktop/src-tauri) | The app itself: settings, the loop that follows the players, windows, tray menu and panel, shortcut, Sing to find, Spotify, True Shuffle, the log, `singline now` and `singline lyrics`. |
| [`apps/desktop/src`](../apps/desktop/src) | The Svelte 5 interface: one page for every window, the texts in `locales`, and the timing and layout helpers in `lib`. |
| [`extensions/browser`](../extensions/browser) | The browser extension for Chrome, Edge and Firefox that reports web players to the app. |
| [`extensions/pi`](../extensions/pi) | The extension for pi that shows the current line in its footer. |

## From the player to the screen

```text
 players                 Rust process                                   windows
 ─────────               ────────────                                   ───────
 MPRIS / SMTC /    ┐
 AppleScript       │     playback thread, every 0.5 s (0.8 s on macOS)
 browser bridge    ├──▶  Engine::choose ─▶ "playback" event ───────────▶ overlay,
 microphone (Sing) ┘          │                                         panel,
                              │ new song                                karaoke
                              ▼
                         LyricsService: own lyrics ─▶ cache ─▶ LRCLIB
                              │
                              └──────────▶ "lyrics" event ─────────────▶
```

1. **Reading the players.** The `playback` thread in [`playback.rs`](../apps/desktop/src-tauri/src/playback.rs) asks every backend for its players: the system's (`media_session::default_backends`), the browser tabs the extension reports, and the microphone player while Sing to find listens. Each answer is a `Reading` with the title, artist, album, duration, position, rate and the controls the player offers.
2. **Choosing one.** `media_session::Engine` picks the player to follow, from the listener's choice of source and whether other players count. The backends have already cleaned up what web players report: [`youtube.rs`](../crates/media-session/src/youtube.rs) turns "Artist - Song (Official Video)" into a song and an artist.
3. **Telling the windows.** The loop sends the state as the `playback` event and keeps `now.json` up to date for the terminal. A new song starts a lookup.
4. **Finding the lyrics.** `LyricsService` in [`lyrics-providers`](../crates/lyrics-providers/src/lib.rs) answers from your own lyrics first, then from the cache, then from LRCLIB: an exact match by title, artist, album and duration, then a search that prefers the same album and versions with word timing. The answer comes back as the `lyrics` event.
5. **Showing them.** The overlay does not wait for the next reading to move on. [`lib/clock.ts`](../apps/desktop/src/lib/clock.ts) carries the position forward between readings and blends small corrections in, so lines and words change on time. Each word lights up from its own timing where the lyrics have it, and from an estimate based on the line otherwise.

Commands go the other way: the windows call Rust through Tauri commands (listed in `main.rs`, wrapped in [`lib/api.ts`](../apps/desktop/src/lib/api.ts)), and player commands such as play, pause and seek reach the followed player through the same loop.

## Threads

| Thread | Does |
|---|---|
| main | Tauri's event loop: windows, tray, menus, the shortcut and the commands from the windows |
| `playback` | Reads the players, chooses one and starts lyrics lookups |
| `settings` | Saves `settings.json` shortly after it changed |
| `browser-bridge` | The HTTP server on `127.0.0.1` for the browser extension, one short thread per request |
| `sing`, `sing-capture`, `sing-meter` | A Sing to find session: the microphone, the level meter, and transcription, search and following |
| Tauri's async runtime | Lyrics lookups, the Spotify Web API and the Whisper model download |

Shared state lives in `AppState` (in `main.rs`) and in a few managed structs, each behind its own `Mutex`. Changes of the settings go through `AppState::update_settings`, which applies what changed to the windows, the tray and the player loop, and sends the new settings to every window. Only values the windows report themselves, such as the overlay's position, take the quiet way, `update_settings_quietly`.

## Windows

The interface is one Vite page. The Rust side opens it with a route per window (`#/overlay`, `#/settings`, `#/welcome`, `#/editor`, `#/stats`, `#/shuffle`, `#/karaoke`, `#/menu`), and [`main.ts`](../apps/desktop/src/main.ts) mounts the matching Svelte component. [`windows.rs`](../apps/desktop/src-tauri/src/windows.rs) creates the windows; [`menu_panel.rs`](../apps/desktop/src-tauri/src/menu_panel.rs) the glass panel that a click on the tray icon opens on macOS and Windows.

The Rust side and the interface share their texts: the tray menu reads the same `locales/en.json` and `locales/de.json` as the Svelte pages.

## Sing to find

[`sing.rs`](../apps/desktop/src-tauri/src/sing.rs) runs a session on its own thread. The microphone feeds a recording (`transcription::capture` and `audio`), voice detection (`vad`) finds stretches of singing, and Whisper turns them into words with a time for each word. Genius turns the words into candidate songs, the usual lookup loads their lyrics, and [`lyrics_core::alignment`](../crates/lyrics-core/src/alignment.rs) decides which song it is and where in it the singer stands. The song then plays through the microphone player, so the overlay follows it like any other player. Only the words go to Genius; the audio stays in memory.

## Where data lives

| File | In | Holds |
|---|---|---|
| `settings.json` | settings folder | The settings |
| `spotify.json`, `sing.json` | settings folder | Your Spotify Client ID and your own Genius token |
| `browser-bridge.json` | settings folder | The paired browsers |
| `singline.sqlite` | data folder | The lyrics cache, your own lyrics, the timing corrections and the listening history |
| `shuffle-state.json` | data folder | The state of True Shuffle |
| `models/` | data folder | The Whisper model for Sing to find |
| `now.json` | cache folder | What plays, for `singline now`, `singline lyrics` and the pi extension |
| `singline.log`, `singline.old.log` | log folder | The log of the last sessions |

The README lists the folders for each system. `SINGLINE_DATA_DIR` puts all of these into one folder, which the end-to-end tests use. The Spotify refresh token is kept in the system's credential store, not in a file.

Online answers stay in the cache for 30 days, plain lyrics for 3 days, and "not found" for a day, so a song played again needs no connection.

## Talking to the outside

| Service | When | What it gets |
|---|---|---|
| LRCLIB | For each new song, unless **Search lyrics online** is off | Title, artist, album and duration |
| Genius | During Sing to find | The words Whisper heard |
| Hugging Face | Once, the first time Sing to find is used | A download of the speech model, checked against its SHA-256 checksum |
| Spotify | Only after you connect it | What the Web API calls need |

The browser extension talks only to the app, on `127.0.0.1`, and only with the token from the pairing.

## Tests

- **Unit tests** in every crate and next to the Svelte helpers (`*.test.ts`, run with Vitest). `crates/lyrics-core/tests/swift_checks.rs` holds the checks of the Swift app, ported one to one.
- **Integration tests** run the clients against local test servers: `lrclib_server.rs`, `genius_server.rs`, `spotify_server.rs` and the model download.
- **End-to-end tests** in [`apps/desktop/e2e`](../apps/desktop/e2e) start the real app on a virtual display with a simulated Spotify (an MPRIS player from `media-session`'s examples), a simulated LRCLIB and Genius, and a simulated browser extension, and compare every line the overlay shows with the playback time. CI runs them on every change.
- **Windows and macOS** are tested by hand with [TESTING.md](TESTING.md).

## Rules the code keeps

- `lyrics-core` stays free of input and output, so all lyrics logic can be tested without a player or a network.
- Tokens, client IDs and lyrics never go into the log; song titles and artists only at debug level. The log can end up in a public issue.
- No song lyrics in the repository, also not in tests: tests use made-up lyrics.
- Every text a listener sees is in both `en.json` and `de.json`; a test checks that both have the same keys.
