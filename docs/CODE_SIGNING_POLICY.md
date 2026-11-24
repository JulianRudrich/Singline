# Code signing policy

Windows releases of Singline are to be signed through the open-source program of the SignPath Foundation. Until the program issues its certificate, the Windows files are unsigned, and Windows asks once before it runs them.

## Signing provider

Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org).

Certificates under this program name the SignPath Foundation as the publisher.

## Roles

| Role | Who | Responsibilities |
|---|---|---|
| Project owner | [Julian Rudrich (@JulianRudrich)](https://github.com/JulianRudrich) | Owns the project and decides who takes part in signing. |
| Author and reviewer | [Julian Rudrich (@JulianRudrich)](https://github.com/JulianRudrich) | Maintains the source code and the build workflows, and reviews every contribution before it can reach a release. |
| Approver | [Julian Rudrich (@JulianRudrich)](https://github.com/JulianRudrich) | Approves each release signing request at SignPath, after checking that it comes from the release build of the expected commit. |

Everyone with access to the repository or to signing uses multi-factor authentication for GitHub and SignPath. Access ends as soon as someone no longer needs it.

## What is signed

Only files built from this repository's source code by its GitHub Actions workflows are signed; nothing from elsewhere is.

| Release asset | Signed files |
|---|---|
| `Singline_<version>_x64-setup.exe` and `Singline-Windows-setup.exe` (installer and in-app update) | The installer and the `singline.exe` app it installs |
| `Singline_<version>_x64_en-US.msi` | The installer and the `singline.exe` app it installs |

The uninstaller the setup writes while it installs is not signed.

## How a release is signed

1. A maintainer starts the [Release workflow](../.github/workflows/release.yml) for a version. It tags the commit and builds every installer on GitHub-hosted runners.
2. The Windows job builds `singline.exe` and submits it to the SignPath project `Singline` with the artifact configuration [`singline-app`](../.signpath/artifact-configurations/singline-app.xml). SignPath checks that the file comes from this repository's workflow before it signs.
3. The job packages the signed app into the two installers and submits them with the artifact configuration [`singline-installers`](../.signpath/artifact-configurations/singline-installers.xml).
4. The approver approves each release signing request at SignPath; the workflow waits for that.
5. The update signature of the setup is made again over the signed file, so installed copies of Singline accept it as an update.

Test builds of `main` are not signed with this certificate.

## Privacy

Singline only connects to other computers to do what it is used for, and says so in the [README](../README.md#privacy):

- **LRCLIB** receives the title, artist, album and duration of the song playing, to find its lyrics. This can be turned off.
- **Genius** receives the words recognised while you use Sing to find, to find the song, when none of the songs you heard fits.
- **Hugging Face** serves the speech model for Sing to find, once, when you first use it.
- **GitHub** is asked whether a newer version exists. This can be turned off.
- **Spotify** is reached only after you connect your own Spotify app.

Singline has no account, no telemetry and no analytics. Its log file stays on your computer unless you send it yourself with **Report a problem**.
