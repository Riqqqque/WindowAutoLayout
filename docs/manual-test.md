# Manual Test Notes

Use these checks for release smoke testing on Windows 11.

## Monitor And Window Detection

- Start WindowAutoLayout and confirm every connected monitor appears with bounds, resolution, primary flag, and scale.
- Put a monitor to the left of primary and confirm negative X coordinates show correctly.
- Disconnect the target monitor and confirm restore returns a missing-monitor result when fallback is disabled.
- Reconnect the monitor and refresh.

## Capture And Restore

- Open OBS and Discord.
- Add or edit app entries so the process names match `obs64.exe` and `Discord.exe`.
- Select each real window in Layout and save it.
- Move both windows away from their saved positions.
- Restore the profile from Dashboard.
- Minimize one window and restore again.
- Put OBS in the system tray with Replay Buffer running, restore the profile, and confirm the existing OBS instance is pulled or woken forward and moved.
- Maximize one window, save the layout, and restore again.
- Enable "Move all matching windows" for a browser and verify multiple matching windows move when intended.

## Launch Flow

- Set executable paths for OBS and Discord.
- Close one app, leave the other open, and restore.
- Close both apps and restore.
- Confirm delayed windows continue to be detected until the timeout.
- Check Logs for launch, match, and move entries.

## Startup, Tray, And Hotkey

- Enable startup restore and confirm the HKCU Run entry is created.
- Close the main window and confirm it hides to tray.
- Restore from the tray menu.
- Lock layout for 30 seconds from tray and move a target window during the lock.
- Press `Ctrl+Alt+L` and confirm the default profile restores.
- Disable the hotkey and confirm it no longer triggers.

## Failure Cases

- Set a bad executable path and restore.
- Use a title rule that does not match and restore.
- Run a target app as administrator while WindowAutoLayout is not elevated and restore.
- Corrupt the config JSON, restart, and confirm the bad file is backed up.
