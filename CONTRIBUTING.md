# Contributing to Singline

Thanks for helping. Singline is a small project, and every bug report, translation and pull request makes a difference.

## Ways to help

- **Report a problem** with the [bug report form](https://github.com/JulianRudrich/Singline/issues/new/choose), or from the app with Settings → About → **Report a problem**. The output of `singline-watch` tells us what your player reports; releases and the test build release include it, and so does every CI run of `main` under Actions.
- **Fix lyrics at the source.** Singline gets its lyrics from [LRCLIB](https://lrclib.net). Missing or badly timed lyrics are best added or corrected there, so every app benefits.
- **Test on Windows and macOS.** [docs/TESTING.md](docs/TESTING.md) is a ten-minute checklist.
- **Translate Singline** into your language; see [Adding a language](#adding-a-language).
- **Write code.** Pick an issue, or open one first for larger changes so we can agree on the approach. [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) explains how the parts fit together.

## Setting up

You need [Rust](https://rustup.rs) 1.89 or later, Node.js 22 and pnpm 10 (`corepack enable` sets up the version the repository asks for). whisper.cpp, which Sing to find uses, also needs CMake, a C++ compiler and libclang:

- **Linux** (Debian and Ubuntu):

  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libasound2-dev libclang-dev cmake
  ```

- **macOS:** the Xcode command line tools (`xcode-select --install`) and CMake (`brew install cmake`).
- **Windows:** Visual Studio 2022 or its Build Tools with **Desktop development with C++**, which includes CMake, and LLVM for libclang (`winget install LLVM.LLVM`). Point `LIBCLANG_PATH` at `C:\Program Files\LLVM\bin`.

To build without Sing to find, and so without CMake and libclang, pass `--no-default-features` to Cargo for the app.

Then install the dependencies and start Singline with the interface reloading on every change:

```bash
pnpm install
```

```bash
pnpm dev
```

To see the overlay without a music player, build a debug version and start it with the built-in sample song:

```bash
pnpm --filter singline-desktop tauri build --debug --no-bundle
```

```bash
./target/debug/singline --demo
```

### Environment variables for development

| Variable | Effect |
|---|---|
| `SINGLINE_DATA_DIR` | Keeps settings, data and the log in this folder instead of the system location |
| `SINGLINE_LRCLIB_URL` | Uses another LRCLIB server, for example a local test server |
| `SINGLINE_DEBUG` | Makes the overlay report every line it shows, for end-to-end checks, and writes debug lines to the log. With it, `--karaoke` and `--menu-panel` open karaoke and the tray panel, at start or from a second launch |
| `SINGLINE_SPOTIFY_ACCOUNTS_URL`, `SINGLINE_SPOTIFY_API_URL` | Use other Spotify servers, for example local test servers |
| `SINGLINE_GENIUS_TOKEN` | At build time: a Genius token built into the app, so Sing to find works without setup |
| `SINGLINE_GENIUS_URL` | Uses another Genius API server, for example a local test server |
| `SINGLINE_MIC_FILE` | Sing to find hears this WAV file instead of the microphone, and starts by itself |
| `SINGLINE_WHISPER_MODEL` | Uses this Whisper model file instead of downloading one |
| `SINGLINE_WHISPER_MODEL_URL` | Downloads the Whisper model from another server |

## Checks

CI runs these on every push. Please run them before you open a pull request:

```bash
cargo fmt --all --check
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
pnpm --filter singline-desktop check
```

```bash
pnpm --filter singline-desktop test
```

```bash
node --test "extensions/browser/test/*.test.js"
```

```bash
node --test "extensions/pi/test/*.test.ts"
```

`cargo test --workspace` also tests the app, which embeds the built interface, so build that first; a plain `cargo test` covers the crates only:

```bash
pnpm --filter singline-desktop build
```

```bash
cargo test --workspace
```

### End-to-end tests

On Linux, the end-to-end tests start the app on a virtual display with a simulated Spotify and a simulated LRCLIB, and compare every line the overlay shows with the playback time. They need `xvfb`, `dbus`, ImageMagick and Python 3:

```bash
sudo apt install xvfb dbus dbus-x11 imagemagick x11-xserver-utils x11-utils espeak-ng
```

```bash
pnpm --filter singline-desktop tauri build --debug --no-bundle
```

```bash
cargo build -p media-session --example fake_mpris_player
```

```bash
bash apps/desktop/e2e/linux-smoke.sh
```

```bash
bash apps/desktop/e2e/linux-youtube.sh
```

```bash
bash apps/desktop/e2e/linux-sing.sh
```

The test for Sing to find plays lines synthesized with espeak-ng instead of the microphone and needs a Whisper model; the top of [`linux-sing.sh`](apps/desktop/e2e/linux-sing.sh) says how to get one, and without it the test is skipped. Screenshots and logs end up in `target/e2e`, `target/e2e-youtube` and `target/e2e-sing`.

### Lint

The [Lint workflow](.github/workflows/lint.yml) checks the spelling with [typos](https://github.com/crate-ci/typos), the links with [lychee](https://lychee.cli.rs), the Rust dependencies with [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) and the workflows with [actionlint](https://github.com/rhysd/actionlint) and [zizmor](https://docs.zizmor.sh). Their settings are in `typos.toml`, `lychee.toml`, `deny.toml` and `.github/zizmor.yml`. With the tools installed, the same checks run locally:

```bash
typos
```

```bash
bash scripts/build-site.sh && lychee --offline './**/*.md' '_site/**/*.html'
```

```bash
cargo deny check
```

```bash
actionlint && zizmor .github
```

## Pull requests

- Keep a pull request to one topic. Screenshots help for anything visible.
- Write commit messages as [Conventional Commits](https://www.conventionalcommits.org): `feat(overlay): …`, `fix(media-session): …`, `docs: …`. Common scopes are `core`, `media-session`, `providers`, `spotify`, `desktop`, `overlay`, `settings`, `extension`, `site` and `ci`. The subject says what the change does, for listeners: the release notes are made from it ([docs/RELEASING.md](docs/RELEASING.md#release-notes)).
- Add every new text to both `en.json` and `de.json`.
- Keep the design: black and see-through with white text, no accent colours, and the motion tokens in `apps/desktop/src/lib/motion.ts`.
- Never commit song lyrics, tokens or other private data, also not in tests or screenshots. Tests use made-up lyrics.

The maintainer reviews every pull request. CI and Lint have to pass before it is merged.

## Adding a language

You can translate without writing code: copy the two English files, translate the texts and open a pull request; the maintainer connects the new language to the app.

1. Copy `apps/desktop/src/locales/en.json` to a file named after the language code, such as `fr.json`, and translate the values. Keep the keys and the placeholders in braces, such as `{player}`.
2. Copy `extensions/browser/_locales/en/messages.json` to `extensions/browser/_locales/fr/messages.json` and translate each `message`.

To connect the language yourself:

1. Add a `language.fr` text with the language's own name to every file in `apps/desktop/src/locales`.
2. In the interface, add the language to `Language` in `apps/desktop/src/lib/types.ts`, to the catalogs in `apps/desktop/src/lib/i18n.ts` and to the choices in `apps/desktop/src/settings/General.svelte`.
3. On the Rust side, add it to `Language` in `apps/desktop/src-tauri/src/settings.rs`, and to `Lang`, `resolve` and `Texts::new` in `apps/desktop/src-tauri/src/i18n.rs`, which also reads the system language.
4. Number and date formats follow the language in `lib/i18n.ts`, `stats/format.ts` and `shuffle/api.ts`.
5. Extend the tests that compare the keys and placeholders of all languages, in `lib/i18n.test.ts` and `src-tauri/src/i18n.rs`.

## License

By contributing, you agree that your contribution is licensed under the [GNU General Public License v3.0 or later](LICENSE), like the rest of Singline. Contributions to the native macOS app in [`apps/macos`](apps/macos) are licensed under the [MIT License](apps/macos/LICENSE).
