<p align="center">
  <img src="src-tauri/icons/icon.png" width="96" alt="WindowAutoLayout icon">
</p>

<h1 align="center">WindowAutoLayout</h1>

<p align="center">
  A quiet Windows tray app that puts OBS, Discord, browsers, and streaming tools back exactly where they belong.
</p>

<p align="center">
  <a href="https://github.com/Riqqqque/WindowAutoLayout/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/Riqqqque/WindowAutoLayout?sort=semver"></a>
  <a href="https://github.com/Riqqqque/WindowAutoLayout/actions/workflows/release.yml"><img alt="Windows build" src="https://github.com/Riqqqque/WindowAutoLayout/actions/workflows/release.yml/badge.svg"></a>
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%2011-0078D4">
  <img alt="Built with" src="https://img.shields.io/badge/built%20with-Tauri%20%2B%20React%20%2B%20Rust-24C8DB">
  <img alt="License" src="https://img.shields.io/badge/license-pending-lightgrey">
</p>

![WindowAutoLayout Apps page](docs/assets/windowautolayout-apps.png)

## Why It Exists

Streaming and gaming setups usually need the same windows in the same places: OBS on one side, Discord on another, a browser where chat or dashboards live, and everything ready after a reboot. Windows can remember some things, but tray apps, delayed startup windows, monitor offsets, and Show Desktop can still scramble a setup.

WindowAutoLayout saves workspace profiles and restores them from the app, tray menu, startup, or a hotkey. It is built for the everyday Windows workflow where pressing one button should bring the setup back, whether the apps are already open, minimized, hidden in the tray, or fully closed.

## What It Does

- Saves app layouts per monitor using monitor-relative coordinates.
- Detects top-level windows with title, class name, process name, PID, bounds, visibility, minimized state, and executable path when Windows exposes it.
- Restores a profile by launching missing apps, waiting for real matching windows, then moving and resizing them with Win32 APIs.
- Pulls minimized and hidden tray windows forward before treating an app as missing.
- Handles OBS in the tray by asking OBS through its tray icon path, then waiting for the real OBS window to repaint before applying the saved layout.
- Keeps a selected profile locked while the lock is on, so Show Desktop, accidental minimize, and accidental moves get snapped back.
- Supports editable app presets, multiple profiles, startup restore, tray restore, logs, JSON import/export, and a global hotkey.
- Stores config and logs locally. No telemetry, accounts, analytics, or background network calls.

## Download

Grab the newest build from [GitHub Releases](https://github.com/Riqqqque/WindowAutoLayout/releases/latest).

Each tagged release publishes:

| Asset | Use |
| --- | --- |
| `WindowAutoLayout_<version>_x64-setup.exe` | Recommended Windows installer |
| `WindowAutoLayout_<version>_x64_en-US.msi` | MSI installer |
| `.sha256` files | Checksums for verifying downloaded installers |

After install, WindowAutoLayout lives under the current Windows user profile and can register startup restore through the current user's `HKCU` Run key.

## Quick Start

1. Install the latest release.
2. Open WindowAutoLayout.
3. Pick the default monitor in Settings, or choose a target monitor on a profile.
4. Add apps from the Apps page. The built-in OBS and Discord presets can be edited.
5. Open and arrange those apps manually.
6. Go to Layout, pick each real window, and save its position.
7. Press Restore from Dashboard, the tray menu, startup restore, or `Ctrl+Alt+L`.

For a complete setup walkthrough, see [docs/usage.md](docs/usage.md).

## OBS And Tray Apps

OBS can stay running in the system tray with Replay Buffer on. For the OBS app entry, keep these enabled:

- `Pull hidden/tray windows`
- `Wake running tray apps`

When OBS is fully hidden in the tray, Windows may report no normal main window. WindowAutoLayout handles that by sending OBS the same tray activation path a manual tray click uses, waiting for OBS to show and repaint, then moving it into the saved layout. That avoids starting a duplicate OBS process and avoids moving the window while OBS is still a blank shell.

More details and recovery checks are in [docs/troubleshooting.md](docs/troubleshooting.md).

## Layout Lock

The lock button repeatedly reapplies the selected profile while it is enabled. It is meant for live setups where the layout should recover after Show Desktop, accidental minimize, dragging, or another app stealing placement. The lock interval is clamped for safety so it stays responsive without turning into a heavy polling loop.

## Startup Restore

Startup restore uses:

```text
"<installed WindowAutoLayout.exe>" --startup-restore
```

The app can start minimized to tray, wait for configured startup delay seconds, restore the default startup profile, launch missing apps, then keep the layout locked if that option is enabled.

## Matching Rules

WindowAutoLayout decides whether a window belongs to an app entry using:

- process name, such as `obs64.exe` or `Discord.exe`
- executable path when Windows exposes it
- optional title rule
- optional class name rule
- title visibility rules
- hidden/minimized settings
- whether the app was just launched by restore

Useful notes:

- OBS may take longer while plugins and docks load.
- Discord uses multiple Electron processes and may appear late.
- Steam can show update or login windows before the main window.
- Browser titles change with tabs.
- Elevated apps may block moves from a normal, non-elevated WindowAutoLayout process.

## Documentation

- [Usage Guide](docs/usage.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Manual Test Notes](docs/manual-test.md)
- [Release Notes](docs/release.md)
- [Example Config](examples/windowautolayout.example.json)

## Build From Source

Requirements:

- Windows 11
- Node.js 20 or newer
- Rust stable with the MSVC toolchain
- Microsoft WebView2 Runtime

```powershell
npm install
npm run check
cd src-tauri
cargo test
cd ..
npm run desktop:build
```

Development:

```powershell
npm run desktop:dev
```

Install or update this PC from the newest local bundle:

```powershell
npm run desktop:install
```

The install helper rebuilds when needed, runs the newest WindowAutoLayout installer, verifies the Windows uninstall entry, finds the installed exe, records the installer hash, and can smoke-launch the installed app.

## Project Layout

```text
src/                  React UI
src-tauri/src/        Rust backend and Win32 restore logic
examples/             Sample config
docs/                 Usage, testing, troubleshooting, and release docs
.github/workflows/    Windows build and release automation
scripts/              Local install/update helper
```

## Release Flow

Every version tag matching `v*` builds the Windows bundles and publishes installer assets to GitHub Releases.

```powershell
git tag v0.1.10
git push origin main
git push origin v0.1.10
```

The workflow validates that the tag matches `package.json`, runs TypeScript and Rust checks, builds the Tauri bundle, generates checksums, uploads CI artifacts, and publishes the release.

## License

License terms have not been finalized yet. Do not redistribute compiled releases or source archives until a license is selected.
