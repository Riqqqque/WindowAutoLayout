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

WindowAutoLayout saves workspace profiles and restores them from the app, tray menu, or Windows startup. It is built for the everyday Windows workflow where pressing one button should bring the setup back, whether the apps are already open, minimized, hidden in the tray, or fully closed.

## What It Does

- Saves app layouts against a hardware-backed monitor identity using monitor-relative physical-pixel coordinates plus captured display and work-area metadata.
- Captures the visible windows on a selected monitor into the active profile.
- Detects top-level windows with title, class name, process name, PID, bounds, visibility, minimized state, and executable path when Windows exposes it.
- Restores a profile by launching missing apps, waiting past splash windows for the real matching surface, then moving and resizing it with Win32 APIs.
- Pulls minimized and hidden tray windows forward before treating an app as missing.
- Handles OBS in the tray through its Qt tray path, applies the saved layout, then runs a bounded presentation recovery so the real interface is painted before restore finishes.
- Uses OpenLaunchDeck's `--show` single-instance handoff to restore its existing startup tray process instead of launching a duplicate.
- Refreshes restored window surfaces without activating them, so apps do not remain blank until clicked.
- Keeps a selected profile protected with an event-driven lock that reacts to Show Desktop and game-to-desktop transitions without a polling loop.
- Reopens the existing tray instance when WindowAutoLayout is launched a second time instead of creating a duplicate process.
- Does not install global mouse hooks, keyboard hooks, raw-input listeners, or system-wide hotkeys.
- Keeps a live `Restore windows now` command and checked automatic-restore state in the tray menu.
- Supports editable app presets, multiple profiles, startup restore, configurable tray left-click behavior, logs, and JSON import/export.
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
4. Open and arrange the apps you want in the profile.
5. On Dashboard, pick the capture monitor and press Capture current layout.
6. Fine-tune app matching on Apps or individual windows on Layout if needed.
7. Press Restore windows now from any app view or the tray menu.
8. Turn on Automatic restore if Show Desktop or returning from a game should recover that profile.

For a complete setup walkthrough, see [docs/usage.md](docs/usage.md).

## Tray Controls

Right-click the tray icon for the current runtime controls:

- `Restore windows now` runs the selected startup/default profile immediately.
- `Automatic restore: On/Off` shows the real state and toggles recovery for the selected automatic profile.
- `Open WindowAutoLayout` opens the existing single app instance.
- `Open activity log` opens the local restore log.
- `Exit` stops the tray process.

Settings can make a normal left-click either open WindowAutoLayout or restore the layout immediately. While a restore is active, the tray label, tooltip, and app header show that state and block duplicate restore requests.

## OBS And Tray Apps

OBS can stay running in the system tray with Replay Buffer on. For the OBS app entry, keep these enabled:

- `Pull hidden/tray windows`
- `Wake running tray apps`

When OBS is fully hidden in the tray, Windows may report no normal main window. WindowAutoLayout handles that by sending OBS the same Qt tray activation path a manual tray click uses, waiting for the main window, applying the saved rectangle, and running a bounded state-verified presentation recovery. That avoids starting a duplicate OBS process and prevents the main frame from being left as a blank shell.

Background restores never fake a click or leave each restored app focused. The generic path uses Windows' non-activating show behavior, applies the saved rectangle, and queues a one-shot surface repaint. OBS uses the bounded recovery above and returns to the previous window when focus did not change. No repaint timer runs after the restore completes.

More details and recovery checks are in [docs/troubleshooting.md](docs/troubleshooting.md).

## Automatic Restore

The automatic control arms a Windows shell-event guard for the selected profile. Show Desktop recovery and post-game recovery can be toggled separately. The guard does not run a timer or repeatedly inspect every window. Background restores never launch missing apps, never force focus, and stop immediately if a game or fullscreen app becomes active again.

Manual and startup restores can launch closed apps when `Launch apps that are closed` is enabled. Turning that setting off still allows an already-running minimized or tray-hidden app to be recovered.

## Game And Input Safety

WindowAutoLayout does not register global hotkeys or low-level mouse/keyboard hooks. Tauri's optional raw device-event stream is explicitly filtered out, the process and restore worker run below normal priority, and the background guard blocks on Windows accessibility events instead of polling. Window movement uses normal Win32 window-management calls and does not synthesize keyboard or mouse input.

The WebView interface is created only while the app window is open. Closing to tray destroys the WebView process tree and leaves the small native tray process running, so normal tray use does not carry a hidden browser runtime through a game.

## Startup Restore

Startup restore uses:

```text
"<installed WindowAutoLayout.exe>" --startup-restore
```

The app can start minimized to tray, wait for the configured startup delay, restore the default profile, optionally launch closed apps, and keep automatic restore armed if it is enabled.

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
- Startup and updater splash windows are ignored until a launch-ready surface appears.
- Steam can show update or login windows before the main window.
- Browser titles change with tabs.
- Elevated apps may block moves from a normal, non-elevated WindowAutoLayout process.

## Documentation

- [Usage Guide](docs/usage.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Manual Test Notes](docs/manual-test.md)
- [Release Notes](docs/release.md)
- [Example Config](examples/windowautolayout.example.json)
- [GitHub Wiki](https://github.com/Riqqqque/WindowAutoLayout/wiki)

## Build From Source

Requirements:

- Windows 11
- Node.js 20 or newer
- Rust stable with the MSVC toolchain
- Microsoft WebView2 Runtime

```powershell
npm ci
npm run check
cd src-tauri
cargo fmt --check
cargo check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cd ..
npm audit --audit-level=moderate
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
git tag v0.1.28
git push origin main
git push origin v0.1.28
```

The workflow validates that the tag matches `package.json`, runs TypeScript, frontend audit, and Rust checks, builds the Tauri bundle, generates checksums, uploads CI artifacts, and publishes the release.

## License

License terms have not been finalized yet. Do not redistribute compiled releases or source archives until a license is selected.
