# Lyrics Overlay

Floating synced lyrics for macOS. Keep the words in view while you work, in Spotify or Apple Music.

![Lyrics Overlay](assets/brand/hero.svg)

## What it does

**Follow the current line** as words brighten with playback, with the next lines nearby. Drag the overlay or snap it to a corner, enable click-through to keep working underneath, and show or hide it with **⌥ ⇧ L**.

**Control playback from the overlay.** Hover for ♥, previous, play/pause, next and a scrubber. Click a lyric line to jump there.

**Follow your player.** Auto follows the app that's playing, or choose Spotify or Apple Music.

**Make it yours.** Adjust type, size, opacity and lyric timing. Choose English or German, launch at login, and auto-hide when nothing plays.

**True Shuffle** (optional, Spotify): play Liked Songs or your own playlists in an honest random order where nothing repeats until everything has played. Needs Spotify Premium and your own free Spotify developer app. [How it works](docs/USAGE.md#true-shuffle).

**Your music.** Listening time, top songs and artists, streaks and a listening clock, recorded only on your Mac. Plus a sleep timer.

## Get it

Download the latest ZIP from [Releases](https://github.com/JulianRudrich/Singline/releases), extract it, and open `Lyrics Overlay.app`. Play a song and allow macOS Automation access to your player when prompted. Lyrics need no player login or API key.

> [!IMPORTANT]
> **Early, unnotarized preview.** This build has no Apple Developer ID signature and may be blocked by Gatekeeper. Only open it if you trust the source; follow [Apple's opening instructions](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unidentified-developer-mh40616/mac). Do not disable Gatekeeper globally.

To build from source, see [Development](docs/DEVELOPMENT.md).

Lyrics come from [LRCLIB](https://lrclib.net/); availability and timing vary by song. Plain lyrics use estimated timing and are marked **~ not synced**. Uncached lookups send track metadata to LRCLIB; Spotify artwork is fetched separately. No analytics. [Privacy and data](docs/PRIVACY.md).

[Usage & troubleshooting](docs/USAGE.md) · [Development](docs/DEVELOPMENT.md) · [Contributing](CONTRIBUTING.md)

Created by [Julian Rudrich](https://github.com/JulianRudrich). The code is licensed under [MIT](LICENSE). Song lyrics belong to their respective rights holders. Not affiliated with Spotify, Apple or LRCLIB.
