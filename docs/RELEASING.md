# Releasing

How the test builds and the releases of Singline are made. Both run on GitHub Actions; nobody builds a release on their own computer.

## Test builds

Every push to `main` runs [CI](../.github/workflows/ci.yml). When the Linux checks and end-to-end tests pass, it builds the app for Windows, macOS and Linux and replaces the [test build release](https://github.com/JulianRudrich/Singline/releases/tag/nightly) with the new files. They keep the same names from build to build, such as `Singline-Windows-setup.exe`, so links to them keep working.

Test builds count up from the next minor version: with 0.3.0 in `tauri.conf.json`, run 74 of CI builds 0.4.0-74. [`scripts/set-version.mjs`](../scripts/set-version.mjs) writes that number into the app before the build.

**Run workflow** on the CI page builds the apps for any branch; they appear under the run's artifacts, not in the test build release.

## Making a release

1. Make sure `main` is green in CI and Lint, and that the [test checklist](TESTING.md) passes on Windows and macOS.
2. Open **Actions → Release → Run workflow**, enter the version without the v (for example `0.4.0`), and tick **dry run** the first time. A dry run builds everything and writes the release notes, but commits, tags and publishes nothing. Its artifacts hold the installers and `RELEASE_NOTES.md`.
3. Run it again without the dry run. The [Release workflow](../.github/workflows/release.yml) then:
   1. checks that the version looks like `0.4.0` and does not exist yet;
   2. writes it into `Cargo.toml`, `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/package.json` and `apps/desktop/src-tauri/tauri.conf.json`;
   3. writes `CHANGELOG.md` and the release notes with [git-cliff](https://git-cliff.org), commits the new version and the changelog as `chore(release): 0.4.0`, tags `v0.4.0` and creates a draft release with the notes;
   4. builds the installers on Windows, macOS and Linux, signs them where it can (below), and uploads them with `singline-watch` and the update packages;
   5. writes `latest.json`, the manifest for automatic updates, publishes the release and starts the [Website workflow](../.github/workflows/pages.yml).
4. Look over the release page and the website.

## Release notes

git-cliff turns the subjects of the commits since the last tag into the release notes, grouped by their [Conventional Commits](https://www.conventionalcommits.org) type ([`cliff.toml`](../cliff.toml)):

| Type | Group |
|---|---|
| `feat` | New features |
| `fix` | Fixes |
| `perf` | Performance |
| `style` | Design |
| `refactor` | Under the hood |
| `docs` | Documentation |
| `test` | Tests |
| `chore`, `ci`, `build` | Maintenance |
| `build(deps)`, `ci(deps)` | Dependencies, the updates from Dependabot |

Commits that do not follow Conventional Commits are left out, and so is `chore(release)`. A good subject therefore reads well on its own: "fix(overlay): keep the lines in view after a seek", not "fix stuff".

## Signing

| What | How | Needs |
|---|---|---|
| Update packages | Tauri signs them, so installed copies accept only updates from this project | the secret `TAURI_SIGNING_PRIVATE_KEY` |
| macOS | Developer ID signature and notarization by Apple | the secrets `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID` |
| Windows | Signed through the SignPath Foundation; the workflow waits until the approver approves each signing request | the variable `SIGNPATH_ORGANIZATION_ID` and the secret `SIGNPATH_API_TOKEN` |

Each kind of signing starts as soon as its secrets exist. Without them the builds still succeed: unsigned, and without update packages. The [code signing policy](CODE_SIGNING_POLICY.md) describes the Windows signing in full.

The secret `SINGLINE_GENIUS_TOKEN` is built into test builds and releases, so Sing to find works without setup. Builds without it ask for a token of your own.

## The website

The [Website workflow](../.github/workflows/pages.yml) publishes `site/`, with the pictures from `docs/images`, to GitHub Pages. It needs Pages to be switched on in the repository settings, with GitHub Actions as the source. The Release workflow starts it after each release; for changes in between, run it by hand. To look at the site locally:

```bash
bash scripts/build-site.sh && python3 -m http.server -d _site
```
