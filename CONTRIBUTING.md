# Contributing to Singline

Thanks for helping. Singline is a small project, and every bug report, translation and pull request makes a difference.

## Ways to help

- **Report a problem** with the [bug report form](https://github.com/JulianRudrich/Singline/issues/new/choose). The output of `singline-watch` (included in every build) tells us what your player reports.
- **Fix lyrics at the source.** Singline gets its lyrics from [LRCLIB](https://lrclib.net). Missing or badly timed lyrics are best added or corrected there, so every app benefits.
- **Test on Windows and macOS.** [docs/TESTING.md](docs/TESTING.md) is a ten-minute checklist.
- **Translate Singline.** All texts live in `apps/desktop/src/locales`. Copy `en.json`, translate the values and open a pull request.
- **Write code.** Pick an issue, or open one first for larger changes so we can agree on the approach.

## Development

The README explains how to [build Singline from source](README.md#building-from-source) and gives an overview of the [project structure](README.md#project-structure). In short:

```bash
pnpm install
```

```bash
pnpm dev
```

`pnpm dev` starts the app with the interface reloading on every change. Start a debug build with `--demo` to see the overlay without a music player.

## Before you open a pull request

Run the checks from the README; CI runs the same on every push:

- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `pnpm --filter singline-desktop check` and `pnpm --filter singline-desktop test`
- `node --test "extensions/browser/test/*.test.js"`
- the end-to-end tests in `apps/desktop/e2e` on Linux

Please also:

- Write commit messages as [Conventional Commits](https://www.conventionalcommits.org): `feat(overlay): …`, `fix(media-session): …`, `docs: …`. Common scopes are `core`, `media-session`, `providers`, `spotify`, `desktop`, `overlay`, `settings`, `extension` and `ci`. The subject says what the change does; the release notes are generated from it.
- Add every new text to both `en.json` and `de.json`.
- Keep the design: black and see-through with white text, no accent colours, and the motion tokens in `apps/desktop/src/lib/motion.ts`.
- Never commit song lyrics, tokens or other private data, also not in tests or screenshots. Tests use made-up lyrics.

## License

By contributing, you agree that your contribution is licensed under the [GNU General Public License v3.0 or later](LICENSE), like the rest of Singline. Contributions to the native macOS app in [`apps/macos`](apps/macos) are licensed under the [MIT License](apps/macos/LICENSE).
