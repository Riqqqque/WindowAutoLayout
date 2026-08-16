# Manual Test Notes

Use these checks for release smoke testing on Windows 11. For restore behavior, prefer the installed app over the dev build.

## Installed App Baseline

- Install the newest local bundle with `scripts\install-current.ps1 -SkipBuild`.
- Confirm the installed exe exists under `%LOCALAPPDATA%\WindowAutoLayout`.
- Confirm the installed file version matches the release version.
- Start the installed app once and confirm it stays alive.
- Start the installed app again and confirm the existing window opens without creating a second process.
- Close the visible window and confirm the tray icon remains when close-to-tray is enabled.
- After closing to tray, confirm WindowAutoLayout has no WebView2 child process and remains near zero CPU while idle.
- Reopen the app and close it again to confirm the WebView lifecycle works repeatedly.
- Disable close-to-tray, close the window, and confirm the process exits instead of becoming unreachable.
- Right-click the tray icon and confirm `Restore windows now` is first and `Automatic restore: On/Off` matches the app header.
- Change the tray left-click setting and confirm both Open WindowAutoLayout and Restore windows now work.

## Monitor And Window Detection

- Start WindowAutoLayout and confirm every connected monitor appears with bounds, resolution, primary flag, and scale.
- Put a monitor to the left of primary and confirm negative X coordinates show correctly.
- Confirm a 4K display and a differently scaled primary display report their physical resolutions and Windows scale values correctly.
- Confirm monitor targets are stored as hardware identities and remain on the same physical panel after Windows display-number reordering.
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
- Compare the final DWM frame bounds to the saved physical-pixel rectangle and confirm every normal window is within one pixel.
- Minimize one window and restore again.
- Turn off `Launch apps that are closed`, hide a running app in the tray, and confirm Restore still recovers it without starting a duplicate process.
- Restore several minimized or tray-hidden apps without clicking them afterward.
- Confirm every restored client area paints fully and remains usable without taskbar activation.
- Confirm a background or layout-lock restore returns to the original foreground window and never runs while a game or fullscreen app is foreground.
- Maximize one window, capture it, and confirm its saved window state remains maximized after restore.
- Change monitor resolution and confirm the captured layout scales to the new physical display bounds.
- Move or resize the taskbar without changing resolution and confirm work-area layouts follow the new usable area.
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

## OpenLaunchDeck Startup Tray Restore

- Enable OpenLaunchDeck launch at startup and confirm its Run entry uses `--background`.
- Leave one OpenLaunchDeck process running with its main window hidden in the tray.
- Restore a profile containing OpenLaunchDeck.
- Confirm the existing OpenLaunchDeck window appears and is moved to the saved rectangle.
- Confirm there is still exactly one long-running `OpenLaunchDeck.exe` process after the `--show` helper exits.
- Confirm Logs include `Asked tray icon to restore hidden running window` or another restore success line.

## Launch Flow

- Set executable paths for OBS and Discord when possible.
- Close one app, leave the other open, and restore.
- Close both apps and restore.
- Confirm delayed windows continue to be detected until the timeout.
- Confirm a small launch or updater splash is ignored until the real main window appears, then verify the settled DWM frame still matches after 30 seconds.
- Confirm app launch failures appear in Logs.
- Confirm running tray apps are woken before a duplicate launch is attempted.
- Turn off `Launch apps that are closed`, close an app fully, and confirm it is reported missing rather than launched.
- Start with the same setting off and confirm startup restore honors it.

## Startup, Tray, And Input Safety

- Enable startup restore and confirm the HKCU Run entry is created.
- Confirm the Run entry includes `--startup-restore`.
- Start the installed exe with `--startup-restore`.
- Confirm the startup delay is honored.
- Confirm the default startup profile restores.
- Close the main window and confirm it hides to tray.
- Restore from the tray menu and confirm its label temporarily changes to `Restoring windows...`.
- Confirm duplicate tray and app restore controls are disabled until that restore finishes.
- Toggle automatic restore from the tray and confirm the app header updates without reopening the app.
- Confirm `Ctrl+Alt+L` is not reserved by WindowAutoLayout.
- Confirm the installed process stays at Below normal priority.
- Confirm there is only one WindowAutoLayout tray process.

## Automatic Restore

- Enable automatic restore for the streaming profile.
- Press Show Desktop and confirm the profile is restored while lock is active.
- Leave a fullscreen app and confirm the profile is restored after returning to the desktop.
- Return to the fullscreen app immediately and confirm the pending restore is cancelled.
- Confirm a closed profile app is not launched by an automatic event restore.
- Confirm disabling the lock stops event restores.
- Sample idle CPU and confirm there is no recurring five-second activity.
- Trigger restore twice quickly and confirm only one restore runs.
- Start a restore while a fullscreen game is foreground and confirm the result says Paused rather than Success.

## Interface

- Check Dashboard, Profiles, Layout, Apps, Logs, and Settings at 1180x760 and the supported minimum 980x640.
- Confirm every control remains visible, text does not overlap, and each page scrolls inside the workspace rather than moving the app shell.
- Overlap saved windows on the Layout canvas and confirm the Selected app menu can still select every app.
- Confirm keyboard focus rings appear on buttons, fields, tabs, selects, and toggles.

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
- Run `cargo test --all-targets` in `src-tauri`.
- Run `cargo clippy --all-targets -- -D warnings` in `src-tauri`.
- Run `npm audit --audit-level=moderate`.
- Run `cargo audit` when a project-local copy is available.
- Run `npm run desktop:build`.
- Install the built app.
- Run the installed-app OBS tray test.
- Push the version tag.
- Confirm the GitHub Actions release workflow passes.
- Confirm GitHub Releases contains setup exe, MSI, and both SHA256 files.
