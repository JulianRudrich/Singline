# Security policy

## Reporting a vulnerability

Please report security problems privately, not in a public issue: open the repository's **Security** tab and choose **Report a vulnerability**, or go straight to the [form for a private report](https://github.com/JulianRudrich/Singline/security/advisories/new). Include what you found, how to reproduce it, and which version and system you used.

If the form is not available, open an issue that asks for a private way to report, without any details of the problem.

You will get an answer within a week. Once the problem is fixed and a new version is out, the report can be published with credit to you, if you like.

## Supported versions

Singline has no stable release yet. Security fixes go into `main`, and with it into the next [test build](https://github.com/JulianRudrich/Singline/releases/tag/nightly). Once there are releases, fixes go into the latest release as well.

## What to look at

These parts of Singline handle data from outside and deserve the most attention:

- **The browser bridge.** The app listens on `127.0.0.1` (ports 17464 to 17466) for the browser extension. It must refuse web pages, other hosts, requests without the token from the pairing, and requests that are too large.
- **The browser extension.** Its scripts run on the music sites it supports and send what plays only to the app on the same computer.
- **The Spotify connection.** The login uses PKCE with a short-lived local listener on port 43821; the refresh token is kept in the system's credential store.
- **Sing to find.** The microphone audio stays in memory on the device and is transcribed there. Only the recognised words go to Genius. The speech model is downloaded from Hugging Face and checked against its SHA-256 checksum before use. A Genius token you enter yourself is kept in `sing.json` in Singline's settings folder.
- **Files and lyrics from outside.** LRC files and lyrics from LRCLIB are parsed and shown, never executed. Imported files are limited to 512 KB.
- **Report a problem.** It opens a GitHub issue in the browser with the last lines of the log, which you read before sending. The log must never contain tokens, client IDs or lyrics.

Singline has no server, no account and no telemetry. It talks to LRCLIB to find lyrics, to Genius and Hugging Face for Sing to find, and to Spotify only after you connect it.
