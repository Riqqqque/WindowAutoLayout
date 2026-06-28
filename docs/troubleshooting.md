# Troubleshooting

Start with Logs inside WindowAutoLayout. Restore writes what it tried, which app it was working on, and why a window was skipped or failed.

User logs live under:

```text
%APPDATA%\com.rique.windowautolayout\logs
```

## OBS Opens White Or Blank

Use the latest release first. OBS tray restore is expected to work when OBS is already running in the tray.

Check the OBS app entry:

- `Process name` is `obs64.exe`
- `Pull hidden/tray windows` is on
- `Wake running tray apps` is on
- `Restore if minimized` is on
- `Detection timeout` is at least 25 seconds

What WindowAutoLayout should do:

1. Detect that `obs64.exe` is already running.
2. Avoid launching a duplicate OBS process.
3. Ask OBS through its tray icon path if OBS has no normal main window.
4. Wait for the real OBS window to show and repaint.
5. Apply the saved layout.

Good log lines look like:

```text
Asked tray icon to restore hidden running window
Applied layout to 1 window(s)
Restore finished with status Success
```

If OBS still does not appear, click the OBS tray icon manually once, save the main OBS window again from Layout, then restore again. That confirms the saved entry points at the real OBS main window instead of a dock panel.

## OBS Says It Is Already Running

This means something tried to start OBS while an OBS process was already alive.

WindowAutoLayout should avoid that when OBS is running in the tray. Confirm:

- only one OBS app entry exists in the profile
- the OBS entry uses `obs64.exe`
- `Wake running tray apps` is enabled
- the installed WindowAutoLayout version is current

If the prompt is already open from an older attempt, close the prompt, leave the main OBS process running, and restore again.

## OBS Dock Moves Instead Of The Main Window

OBS has dock windows such as Stats, Chat, and Stream Information. Those can look like valid windows to Windows.

Fix:

1. Open the real OBS main window.
2. Go to Layout.
3. Pick the main OBS window, not a dock window.
4. Save the layout.
5. Restore the profile.

When no title rule is set, WindowAutoLayout avoids obvious OBS tool and dock windows where possible. If a custom title rule is too broad, tighten it or remove it.

## App Does Not Launch

Check the app entry:

- executable path is correct, if one is set
- process name matches the real executable name
- arguments are valid
- working directory exists, if one is set

If the executable path is blank, WindowAutoLayout tries normal Windows discovery paths, app paths, Start Menu shortcuts, common install folders, and `PATH`.

## App Is Running But No Window Is Found

This usually means the app is in the tray, still loading, elevated, or showing a splash/update/login window.

Try:

- increase `Detection timeout`
- enable `Pull hidden/tray windows`
- enable `Wake running tray apps`
- remove an overly strict title rule
- save the layout from the real main window again
- run WindowAutoLayout elevated only if the target app is also elevated

## Window Moves To The Wrong Monitor

WindowAutoLayout saves positions relative to the target monitor. If monitor IDs change after unplugging, driver updates, or display rearranges, the saved target can point somewhere unexpected.

Fix:

1. Go to Settings and refresh monitors.
2. Pick the default monitor again.
3. Check the profile target monitor.
4. Check app-level target monitors.
5. Save the layout again if the monitor arrangement changed.

## Show Desktop Still Hides Windows

The Windows Show Desktop command can minimize windows. Layout lock is the recovery mechanism.

Turn on layout lock for the profile. While the lock is active, WindowAutoLayout watches the managed windows and restores the profile when one moves, minimizes, hides, or disappears.

## Hotkey Does Nothing

Check:

- hotkey is enabled in Settings
- the accelerator is valid
- another app is not already using the same hotkey
- WindowAutoLayout is still running in the tray

Try changing the hotkey, saving, then changing it back.

## Startup Restore Does Not Run

Check:

1. Settings has `Start with Windows` enabled.
2. Settings has `Restore on launch` enabled.
3. The intended profile is marked as the default startup profile.
4. The current user's Run key contains `WindowAutoLayout`.

The startup command should include:

```text
--startup-restore
```

## Config Looks Broken

WindowAutoLayout stores config as JSON under:

```text
%APPDATA%\com.rique.windowautolayout\config.json
```

If the file cannot be parsed, WindowAutoLayout backs it up with a timestamp and creates a clean config. You can use Settings import/export to move a known-good config between machines.

## Checks For A Release Build

Use these before trusting a release:

```powershell
npm run check
cd src-tauri
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
cd ..
npm audit --audit-level=moderate
npm run desktop:build
powershell -ExecutionPolicy Bypass -File scripts\install-current.ps1 -SkipBuild
```

For OBS specifically, test with OBS hidden in the tray, restore from the installed app, and confirm:

- OBS process count stays at one
- no duplicate already-running prompt appears
- OBS main window is visible
- the restored OBS window is not blank or white
