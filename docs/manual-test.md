# Manual Test Notes

Use these checks for release smoke testing on Windows 11. For restore behavior, prefer the installed app over the dev build.

## Installed App Baseline

- Install the newest local bundle with `scripts\install-current.ps1 -SkipBuild`.
- Confirm the installed exe exists under `%LOCALAPPDATA%\WindowAutoLayout`.
- Confirm the installed file version matches the release version.
- Start the installed app once and confirm it stays alive.
- Close the visible window and confirm the tray icon remains when close-to-tray is enabled.

## Monitor And Window Detection

- Start WindowAutoLayout and confirm every connected monitor appears with bounds, resolution, primary flag, and scale.
- Put a monitor to the left of primary and confirm negative X coordinates show correctly.
- Disconnect the target monitor and confirm restore returns a missing-monitor result when fallback is disabled.
- Reconnect the monitor and refresh.
- Confirm window detection shows title, class name, process name, PID, bounds, and visible/minimized state.

## Capture And Restore

- Open OBS and Discord.
- Add or edit app entries so process names match `obs64.exe` and `Discord.exe`.
- Select each real window in Layout and save it.
- Move both windows away from their saved positions.
- Restore the profile from Dashboard.
- Minimize one window and restore again.
- Maximize one window, save the layout, and restore again.
- Enable `Move all matching windows` for a browser and verify multiple matching windows move when intended.
- Check Logs for applied layout counts.

## OBS Tray Restore

This is the important streaming test.

- Keep OBS running.
- Enable OBS tray settings inside OBS if needed.
- Minimize OBS to the system tray.
- Confirm there is still one `obs64.exe` process.
- Restore the WindowAutoLayout profile from the installed app.
- Confirm OBS comes back as the main OBS window.
- Confirm OBS does not show a blank or white window.
- Confirm no `OBS is already running` prompt appears.
- Confirm the OBS process count stays at one.
- Confirm Logs include `Asked tray icon to restore hidden running window` or another restore success line.

## Launch Flow

- Set executable paths for OBS and Discord when possible.
- Close one app, leave the other open, and restore.
- Close both apps and restore.
- Confirm delayed windows continue to be detected until the timeout.
- Confirm app launch failures appear in Logs.
- Confirm running tray apps are woken before a duplicate launch is attempted.

## Startup, Tray, And Hotkey

- Enable startup restore and confirm the HKCU Run entry is created.
- Confirm the Run entry includes `--startup-restore`.
- Start the installed exe with `--startup-restore`.
- Confirm the startup delay is honored.
- Confirm the default startup profile restores.
- Close the main window and confirm it hides to tray.
- Restore from the tray menu.
- Press `Ctrl+Alt+L` and confirm the selected/default profile restores.
- Disable the hotkey and confirm it no longer triggers.

## Layout Lock

- Enable layout lock for the streaming profile.
- Move a target window during the lock and confirm it snaps back.
- Press Show Desktop and confirm the profile is restored while lock is active.
- Minimize OBS during the lock and confirm it is pulled back when OBS settings allow it.
- Confirm the lock stops after its duration or when disabled manually.
- Watch CPU usage briefly and confirm the lock is not causing heavy idle load.

## Failure Cases

- Set a bad executable path and restore.
- Use a title rule that does not match and restore.
- Run a target app as administrator while WindowAutoLayout is not elevated and restore.
- Corrupt the config JSON, restart, and confirm the bad file is backed up.
- Disable hidden-window pulling for OBS, hide OBS to tray, restore, and confirm the failure is clear.
- Disconnect the profile monitor, restore, and confirm the monitor-missing behavior is respected.

## Release Verification

- Run `npm run check`.
- Run `cargo fmt --check` in `src-tauri`.
- Run `cargo test` in `src-tauri`.
- Run `npm run desktop:build`.
- Install the built app.
- Run the installed-app OBS tray test.
- Push the version tag.
- Confirm the GitHub Actions release workflow passes.
- Confirm GitHub Releases contains setup exe, MSI, and both SHA256 files.
