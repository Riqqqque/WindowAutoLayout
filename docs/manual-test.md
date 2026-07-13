# Manual Test Notes

Use these checks for release smoke testing on Windows 11. For restore behavior, prefer the installed app over the dev build.

## Installed App Baseline

- Install the newest local bundle with `scripts\install-current.ps1 -SkipBuild`.
- Confirm the installed exe exists under `%LOCALAPPDATA%\WindowAutoLayout`.
- Confirm the installed file version matches the release version.
- Start the installed app once and confirm it stays alive.
- Start the installed app again and confirm the existing window opens without creating a second process.
- Close the visible window and confirm the tray icon remains when close-to-tray is enabled.
- Disable close-to-tray, close the window, and confirm the process exits instead of becoming unreachable.

## Monitor And Window Detection

- Start WindowAutoLayout and confirm every connected monitor appears with bounds, resolution, primary flag, and scale.
- Put a monitor to the left of primary and confirm negative X coordinates show correctly.
- Disconnect the target monitor and confirm restore returns a missing-monitor result when fallback is disabled.
- Reconnect the monitor and refresh.
- Confirm window detection shows title, class name, process name, PID, bounds, and visible/minimized state.

## Capture And Restore

- Open OBS and Discord.
- Arrange the windows on one monitor.
- On Dashboard, pick that monitor and press Capture current layout.
- Confirm the profile app list now matches the visible windows on that monitor.
- Confirm the profile target monitor changed to the captured monitor.
- Select each real window in Layout and save it when testing per-window capture.
- Move both windows away from their saved positions.
- Restore the profile from Dashboard.
- Minimize one window and restore again.
- Maximize one window, capture it, and confirm its saved window state remains maximized after restore.
- Change monitor resolution and confirm oversized saved bounds are constrained to the current display.
- With three displays connected, confirm nearest fallback chooses the display closest to the saved profile size.
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

## Startup, Tray, And Input Safety

- Enable startup restore and confirm the HKCU Run entry is created.
- Confirm the Run entry includes `--startup-restore`.
- Start the installed exe with `--startup-restore`.
- Confirm the startup delay is honored.
- Confirm the default startup profile restores.
- Close the main window and confirm it hides to tray.
- Restore from the tray menu.
- Confirm `Ctrl+Alt+L` is not reserved by WindowAutoLayout.
- Confirm the installed process stays at Below normal priority.
- Confirm there is only one WindowAutoLayout tray process.

## Layout Lock

- Enable layout lock for the streaming profile.
- Press Show Desktop and confirm the profile is restored while lock is active.
- Leave a fullscreen app and confirm the profile is restored after returning to the desktop.
- Return to the fullscreen app immediately and confirm the pending restore is cancelled.
- Confirm a closed profile app is not launched by an automatic event restore.
- Confirm disabling the lock stops event restores.
- Sample idle CPU and confirm there is no recurring five-second activity.
- Trigger restore twice quickly and confirm only one restore runs.
- Start a restore while a fullscreen game is foreground and confirm the result says Paused rather than Success.

## Config Import

- Import a valid exported config and confirm it remains editable before saving.
- Import malformed JSON and confirm the current config stays unchanged.
- Import duplicate or empty profile/app IDs and confirm they are repaired without losing entries.
- Import extreme delay and rectangle values and confirm they are constrained to safe bounds.

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
