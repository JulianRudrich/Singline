# Singline for your browser

This extension lets Singline show synced lyrics for what you play in your browser: YouTube Music, YouTube, Spotify, Apple Music and Amazon Music on the web. It keeps working while the tab is in the background.

It reads what the tab plays (title, artist, album, cover, position and speed) and sends it to the Singline app on the same computer. Nothing leaves your computer: the extension only talks to `127.0.0.1`, and only after you paired it with Singline once.

One code base runs in Chrome, Edge and Firefox (128 or newer).

## Install it

The extension is not in the browser stores yet. Load it from this folder: download [the repository as a ZIP file](https://github.com/JulianRudrich/Singline/archive/refs/heads/main.zip), unpack it, and use the folder `extensions/browser` inside.

**Chrome, Edge, Brave, Vivaldi**

1. Open `chrome://extensions` (Edge: `edge://extensions`).
2. Turn on **Developer mode**.
3. Click **Load unpacked** and choose the folder `extensions/browser`.
4. Pin the Singline icon to the toolbar, so the pairing is one click away.

**Firefox**

1. Open `about:debugging#/runtime/this-firefox`.
2. Click **Load Temporary Add-on…** and choose `extensions/browser/manifest.json`.
3. Firefox removes temporary add-ons when it quits; load it again after a restart.
4. If the toolbar icon shows a dot, click it and allow the extension on the music sites.

After an update of these files, click the reload arrow of the extension (Chrome, Edge) or **Reload** (Firefox), and reload the music tabs.

## Pair it with Singline

1. Start Singline and open its settings. Under **Browser extension** you see a six-digit code.
2. Click the Singline icon in the browser toolbar, type the code and click **Pair**.
3. The popup now says **Connected to Singline**. Play a song: the lyrics appear in Singline.

You pair once per browser. Singline lists every paired browser in its settings; **Remove** there, or **Unpair this browser** in the popup, ends the pairing.

The code changes after each pairing and after five wrong tries, and Singline only accepts it while its settings show it.

## Which players Singline follows

- **YouTube Music** counts like Spotify, Apple Music and Amazon Music: Singline follows it automatically, and you can also pick it as the playback source.
- **YouTube** plays more than music, so Singline follows it only with **Also follow other players** turned on. It turns a video into a song before it searches for lyrics: "Artist - Song (Official Video)" on "ArtistVEVO" becomes "Song" by "Artist".

Without the extension, Singline still sees browser tabs through the system on Windows and Linux, but it does not always learn which site plays, and on macOS it does not see them at all.

## How it works

- `content/page.js` runs in the page, reads the `<video>` or `<audio>` element that plays and `navigator.mediaSession.metadata`, and reports about once a second while playing and on every change.
- `content/relay.js` passes the reports to the background and Singline's commands back to the page.
- `background.js` with `lib/client.js` and `lib/protocol.js` pairs with Singline and sends the reports to it. Singline answers with commands (play/pause, next, previous, seek), which the tab carries out where the site allows.
- `popup/` shows the connection and pairs.

Singline listens on the first free port of 17464, 17465 and 17466, on `127.0.0.1` only. It refuses requests from web pages, requests without the token from the pairing, and requests that are too large.

## Tests

```sh
node --test "extensions/browser/test/*.test.js"
```

The tests and Singline's own tests read the same example message in `test/fixtures/update.json`, so both sides agree on the format.
