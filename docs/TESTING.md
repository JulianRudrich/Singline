# Testing on Windows and macOS

Linux is tested automatically in CI. Windows and macOS need a real device. One pass takes about ten minutes per system. Please report the results in an [issue](https://github.com/JulianRudrich/Singline/issues/new) with the output of `singline-watch`; screenshots help with anything that looks wrong.

## Get the builds

1. Download the newest build from the [test build release](https://github.com/JulianRudrich/Singline/releases/tag/nightly), or open **Actions → CI** and choose the newest successful run on `main`. Every commit on `main` produces test builds; **Run workflow** starts a new run.
2. When the run has finished, download from **Artifacts**:
   - `singline-macOS`: the `.dmg` and `singline-watch-macos.tar.gz`
   - `singline-Windows`: the installer `…-setup.exe` (or `.msi`) and `singline-watch.exe`
   - `singline-Linux`: a `.deb`, an `.AppImage` and `singline-watch-linux.tar.gz`

The builds are not signed yet:

- **macOS:** drag the app from the `.dmg` into Applications. If macOS refuses to open it, go to **System Settings → Privacy & Security** and choose **Open Anyway** (on macOS 14 and earlier, right-click the app and choose **Open**). If it says the app is damaged, run `xattr -dr com.apple.quarantine /Applications/Singline.app`. For the tool, run `tar -xzf singline-watch-macos.tar.gz` and `xattr -d com.apple.quarantine singline-watch`.
- **Windows:** if SmartScreen appears, choose **More info** → **Run anyway**.

## 1. What the players report (measurement B)

Play a song in **Spotify**, **Apple Music** and **Amazon Music**, one after another, whichever is available on the system, and keep the tool running:

- macOS (Terminal): `./singline-watch`
- Windows (PowerShell, in the download folder): `.\singline-watch.exe`

The first time, macOS asks whether Terminal may control Spotify or Music: allow it.

Let it run for about 20 seconds per player, seek once in the player and pause once. Copy the output into a text file. What matters: are title, artist and album correct? Does the position keep up (`drift` close to 0)? Which `controls` are listed?

On Windows, also play a song in a **browser**, for example YouTube Music or Amazon Music on the web, and check whether it appears.

While a player is running, test the commands:

```bash
./singline-watch --command playpause
```

```bash
./singline-watch --command seek=60
```

```bash
./singline-watch --command next
```

## 2. The app

After starting, the overlay appears in the top right corner and an icon appears in the tray (the menu bar on macOS). At the first start the welcome window opens as well.

- [ ] Welcome window (first start; to see it again, set `"welcomed": false` in `settings.json`): does the tray icon in the picture match where it really is? Choose Spotify and play a song: does it say that Singline follows the song? On macOS, deny the access once: does it offer System Settings? After **Done**, or after closing it, does it stay closed at the next start?
- [ ] Play a Spotify song: do the lyrics appear, and does the highlighted line match the singing?
- [ ] If the song has word timing: do the words light up one by one?
- [ ] Pause, seek and change the song in the player: does the overlay follow within a second?
- [ ] The same with Apple Music, and with Amazon Music if it reported data in part 1.
- [ ] Does the overlay stay above other windows, including a full-screen video and on another desktop? (measurement A)
- [ ] Click into another app: does that app keep the focus?
- [ ] Drag the overlay, quit and restart the app: is the position remembered?
- [ ] Tray menu: hide and show, click-through, corners, playback source.
- [ ] Alt+Shift+L (⌥⇧L on macOS) hides and shows the overlay.
- [ ] Settings (the button in the overlay or the tray menu): layouts, font size, opacity, width and height take effect immediately.
- [ ] Lyrics only (tray menu or Settings → Appearance): do the song, the buttons, the microphone and the timeline disappear, also under the pointer, and come back when it is turned off?
- [ ] Song & timing: ±0.25 s changes the timing audibly; after switching to another song and back, the value is still there.
- [ ] Own lyrics: import an `.lrc` file, then open the editor, stamp a few lines with Ctrl+Shift+T (⌘⇧T on macOS) and save. Does the overlay show them at once? Remove them again.
- [ ] Switch the language between English and German: do the overlay, settings and tray menu change?
- [ ] Open at login: turn it on, log out and log back in.
- [ ] Open the app a second time while it is running (on macOS from Applications or with a click on the Dock icon): do the settings open, and is click-through off?
- [ ] macOS: while the settings are open, is Singline in the Dock, in the app switcher (⌘Tab) and in Force Quit (⌥⌘Esc)? Does it leave the Dock when the last of its windows closes? Does ⌘Q quit it?
- [ ] Settings → About → **Report a problem**: does the browser open a new issue with the system, its version, the Singline version, the player and the last lines of the log filled in? **Show** next to **Log file**: does Finder or Explorer open with `singline.log` selected?
- [ ] **Quit Singline** at the bottom of the settings quits the app.
- [ ] In the terminal (see the README): while a song plays, does `singline now` print the line being sung (macOS: `/Applications/Singline.app/Contents/MacOS/singline now`)? Does `singline lyrics` fill the terminal with the lyrics, the current line lighting up, and does q leave? With the pi extension installed, does pi's footer show the line, word by word, and does `/singline` hide it? After quitting Singline, does the footer go quiet within half a minute?
- [ ] macOS: Settings → General → **Show in Dock**: does Singline stay in the Dock after the settings close, does a click on the Dock icon open the settings, and does it leave the Dock when the setting is off again?
- [ ] Sing to find (tray menu or the microphone in the overlay): the first time, does the system ask for the microphone and the speech model download show its progress? Sing or play a few lines of a well-known song: is it found within about ten seconds, and does the overlay start at the line you sang and keep up? Deny the microphone in the system settings and try again: does the overlay say where to allow it? Turn Sing to find off in Settings → General: do the microphone in the overlay and the menu entries disappear?

Note ✅ for each item, or what happened instead.

## Demo without a player

`--demo` plays a built-in song with word timing:

- macOS: `/Applications/Singline.app/Contents/MacOS/singline --demo`
- Windows: `"%LOCALAPPDATA%\Singline\singline.exe" --demo` (after installing the `.msi`: `"C:\Program Files\Singline\singline.exe" --demo`)
