# Feature parity

Before its first release, Singline must offer every feature of the native macOS app (Linory 0.3.1, contributed in [#1](https://github.com/JulianRudrich/Singline/pull/1)) on every system. Before that release, this list is compared again with the latest version of Linory.

**Legend:** ✅ built and tested · 🧪 built, not yet tested on a device · ⬜ open · — not available on this system

**Columns:** Lin = Linux, Win = Windows, Mac = macOS. Linux is tested automatically on a virtual display with a simulated player.

## Overlay

| Feature | Lin | Win | Mac | Notes |
|---|:-:|:-:|:-:|---|
| Floats above other windows on every desktop | ✅ | 🧪 | 🧪 | macOS above full-screen apps: measurement A |
| Never takes the focus from other apps | 🧪 | 🧪 | 🧪 | |
| Dragging, four corners, remembered position, several displays | 🧪 | 🧪 | 🧪 | |
| Click-through, remembered; opening the app again turns it off | 🧪 | 🧪 | 🧪 | macOS: also from Applications or the Dock |
| Shortcut to show and hide, changeable in the settings | 🧪 | 🧪 | 🧪 | Linux: X11 only for now |
| Three layouts, font size, opacity and size | ✅ | 🧪 | 🧪 | Linory has one fixed layout |
| Lyrics only, without song, controls and timeline | ✅ | 🧪 | 🧪 | Not in Linory |
| Welcome at the first start: where the icon is, player, permissions, extension | ✅ | 🧪 | 🧪 | Not in Linory |
| Lyrics in the terminal: `singline now`, `singline lyrics` and the pi extension | ✅ | 🧪 | 🧪 | Windows: the pi extension only. Not in Linory |
| Word-by-word highlight from real word timing | ✅ | 🧪 | 🧪 | Linory mostly estimates |
| Estimated word highlight for line-synced lyrics | ✅ | 🧪 | 🧪 | |
| Scrolling lines and smooth song changes | ✅ | 🧪 | 🧪 | |
| Splitting long lines | ✅ | 🧪 | 🧪 | |
| Cover art | 🧪 | 🧪 | 🧪 | Windows: thumbnail of the media session; macOS: Spotify only so far |
| Hiding while nothing plays | 🧪 | 🧪 | 🧪 | |

## Mini-player

| Feature | Lin | Win | Mac | Notes |
|---|:-:|:-:|:-:|---|
| Play/pause, next, previous and seeking (commands) | ✅ | 🧪 | 🧪 | |
| Controls and a seek bar with times | ✅ | 🧪 | 🧪 | macOS: hover while the app is inactive needs checking |
| Jumping to a line by clicking it | ✅ | 🧪 | 🧪 | |
| Chorus markers | ✅ | 🧪 | 🧪 | |
| ♥ (Spotify) | ✅ | ⬜ | 🧪 | Windows does not report Spotify's track IDs |
| Open in Spotify | ⬜ | ⬜ | ⬜ | |

## Sync and lyrics

| Feature | Lin | Win | Mac | Notes |
|---|:-:|:-:|:-:|---|
| Position carried forward between readings, jitter smoothed | ✅ | 🧪 | 🧪 | Lines change about 10 ms after their timestamp |
| Immediate reaction to song changes through system notifications | ⬜ | ⬜ | ⬜ | Today the players are read every 0.5 s (macOS 0.8 s) |
| Timing correction per song and globally | ✅ | 🧪 | 🧪 | Linory: global only |
| LRCLIB exact match, then a search that prefers the same album | ✅ | ✅ | ✅ | Tested independently of the system |
| Local cache | ✅ | ✅ | ✅ | |
| Plain lyrics, marked as not synced | ✅ | ✅ | ✅ | Timing follows the line length instead of even spacing |
| Instrumentals recognised | ✅ | ✅ | ✅ | |
| Own lyrics: storage | ✅ | ✅ | ✅ | |
| Own lyrics: LRC import and editor | ✅ | 🧪 | 🧪 | From the 0.2 preview |

## Players

| Player | Lin | Win | Mac | Notes |
|---|:-:|:-:|:-:|---|
| Spotify (desktop app) | ✅ | 🧪 | 🧪 | Linux tested with a simulated player |
| Apple Music | 🧪 | 🧪 | 🧪 | Windows: the combined "Artist — Album" field is split; Linux: web player |
| YouTube Music | ✅ | 🧪 | 🧪 | Browser tabs through the extension; Windows also the web app; Linux also without the extension |
| Amazon Music | 🧪 | 🧪 | 🧪 | Windows: measurement B; Linux and macOS: web player |
| Automatic or fixed source | ✅ | ✅ | ✅ | |
| Other players (browsers, VLC and more) | ✅ | 🧪 | 🧪 | macOS: web players through the browser extension |
| Browser extension (Chrome, Edge, Firefox) | ✅ | 🧪 | 🧪 | Linux tested with a simulated extension; not yet in the browser stores |

## More features

| Feature | Lin | Win | Mac | Notes |
|---|:-:|:-:|:-:|---|
| English and German, following the system language | ✅ | 🧪 | 🧪 | |
| Open at login | 🧪 | 🧪 | 🧪 | |
| Tray menu | 🧪 | 🧪 | 🧪 | |
| Glass panel from the tray icon: current song, controls, quick switches | — | 🧪 | 🧪 | Linux reports no clicks on the tray icon, so it opens the menu |
| Sleep timer | 🧪 | 🧪 | 🧪 | |
| Karaoke: fullscreen lyrics on a chosen display, countdowns, intro card | 🧪 | 🧪 | 🧪 | Not in Linory; Linux checked on a virtual display |
| Listening statistics | ✅ | 🧪 | 🧪 | For every player |
| Spotify connection, True Shuffle and top lists | ✅ | 🧪 | 🧪 | Linux tested against a simulated Spotify |
| Installers | 🧪 | 🧪 | 🧪 | CI builds .deb, AppImage, .exe, .msi and .dmg |
| Homebrew | — | — | ⬜ | |
| Automatic updates | ⬜ | ⬜ | ⬜ | |
