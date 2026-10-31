# Heartwood for Windows

The Windows app of [Heartwood](https://heartwood-game.web.app): the game in a desktop window, with an
installer and updates. It carries the same files as the website (bundled, so single-player works offline)
in a Tauri window, the way the iPhone, iPad and Mac apps carry them.

**Download:** [Heartwood-Setup.exe](https://github.com/boomcrop/heartwood-windows/releases/latest/download/Heartwood-Setup.exe)
(or the [.msi](https://github.com/boomcrop/heartwood-windows/releases/latest/download/Heartwood.msi)). Windows 10 and 11.
The installer isn't code-signed yet: if Windows says "Windows protected your PC", click **More info**, then **Run anyway**.

## How a release is made

Push a tag like `v1.0.1`. The workflow in `.github/workflows/release.yml` builds on Windows, installs the
result and checks that it starts, then publishes a release with the installers and `latest.json`. Installed
copies read `latest.json` at launch and offer the update.

## Building

The game's source and the website build live in two private repos next to this one (`heartwood`,
`heartwood-deploy`). `tools/sync_web.sh` builds the website and copies it into `web/`; `npx tauri build`
then makes the installers (on Windows).
