# Heartwood for Windows and Mac

The desktop app of [Heartwood](https://heartwood-game.web.app): the game in its own window, with an
installer and updates. It carries the same files as the website (bundled, so single-player works offline)
in a Tauri window, the way the App Store apps for iPhone, iPad and Mac carry them.

**Windows:** [Heartwood-Setup.exe](https://github.com/boomcrop/heartwood-windows/releases/latest/download/Heartwood-Setup.exe)
(or the [.msi](https://github.com/boomcrop/heartwood-windows/releases/latest/download/Heartwood.msi)). Windows 10 and 11.
The installer isn't code-signed yet: if Windows says "Windows protected your PC", click **More info**, then **Run anyway**.

**Mac:** [Heartwood.dmg](https://github.com/boomcrop/heartwood-windows/releases/latest/download/Heartwood.dmg).
macOS 12 or later, Apple silicon and Intel. Open it and drag Heartwood into Applications. The app isn't
notarized by Apple yet: if macOS says it can't verify Heartwood, click **Done**, open **System Settings >
Privacy & Security**, scroll down and click **Open Anyway**.

## How a release is made

Push a tag like `v1.1.1`. The workflow in `.github/workflows/release.yml` builds on Windows and on a Mac,
installs each result and checks that it starts, then publishes a release with the installers, the disk image
and `latest.json`. Installed copies read `latest.json` at launch and offer the update.

## Building

The release build takes the game from the live website (deploy it first): `tools/fetch_web.sh` downloads the
files the site lists in `app-files.txt` into `web/`, so the app carries exactly what browser players get. On a
Mac with the game's private repos beside this one, `tools/sync_web.sh` builds the website locally instead.
`npx tauri build` then makes the installers on Windows, or the app and disk image on a Mac
(`npx tauri build --target universal-apple-darwin` for one app that runs on every Mac).
`src-tauri/tauri.macos.conf.json` holds the Mac's own settings.
