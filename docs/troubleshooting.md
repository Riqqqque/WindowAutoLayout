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
3. Ask OBS through its tray icon path when the main window is hidden or minimized.
4. Wait for the real OBS window to show and repaint.
5. Apply the saved layout.
6. Queue a no-focus surface relayout and child repaint.

The final no-focus refresh is shared by all restored apps. It prevents stale dock cuts and blank client areas without adding any background repaint loop.

Minimized OBS windows use the Qt tray restore handler first. Directly changing the Win32 show state can expose an OBS frame before Qt has rebuilt its docks, which is the white-client symptom this path avoids.

OBS then receives one bounded presentation recovery. WindowAutoLayout verifies that Windows actually reached the minimized state, restores the window, verifies that it is no longer minimized, and gives Qt time to present the rebuilt interface. The recovery is blocked while a game or fullscreen app is foreground, and the previous window is restored only when focus stayed on OBS during the pulse.

Good log lines look like:

```text
Asked tray icon to restore hidden running window
Applied layout to 1 window(s)
Restore finished with status Success
```

If OBS still does not appear, click the OBS tray icon manually once, save the main OBS window again from Layout, then restore again. That confirms the saved entry points at the real OBS main window instead of a dock panel.

## Restored Window Is Blank Until Clicked

WindowAutoLayout uses an asynchronous exposure cycle for windows coming out of the tray or a minimized state, then performs one bounded resize and queues a full client repaint. OBS also gets the state-verified presentation recovery described above. No mouse click or key press is synthesized, and there is no repaint activity after the restore finishes.

If one app still remains blank, confirm its saved entry targets the real main window rather than a splash, helper, dock, or GPU overlay window. Capture that app again while its main interface is visible, then inspect the matched title and class in Layout.

## Restore Button Or Tray State Looks Stuck

The app header and tray menu should both show `Restoring` only while one restore is active. Duplicate restore controls are intentionally disabled during that time.

If the state does not clear, open the activity log and look for a target app that stopped responding during launch or window recovery. Exit WindowAutoLayout from the tray and reopen it; the single-instance guard prevents a second background copy from taking over the tray state.

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

`Launch apps that are closed` may be turned off. That prevents new process launches but does not prevent WindowAutoLayout from recovering an app that is already running in the tray.

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

WindowAutoLayout saves positions relative to a hardware-backed monitor identity. Version 0.1.28 migrates older `DISPLAY1` and `DISPLAY2` targets and uses saved canvas dimensions when Windows has reassigned those volatile names.

Fix:

1. Go to Settings and confirm the monitor model and resolution shown as the default.
2. Refresh app data after reconnecting or rearranging displays.
3. Check the profile target monitor.
4. Check app-level target monitors.
5. Save the layout again only if the physical workspace itself changed.

Captured layouts include the display's physical resolution and usable work area. A resolution or taskbar change is mapped into the new area. On an unchanged display, saved coordinates remain exact instead of being rescaled.

## Window Is A Few Pixels Off

Normal-window restores are checked repeatedly against the DWM frame bounds with a one-pixel tolerance. If an app keeps overriding its own size or location, the restore result reports both the expected and actual rectangle instead of claiming success.

Check whether the app is elevated, enforcing a minimum size, restoring its own session, or using a saved maximized state. Capture the settled main window again after the app has finished loading.

## Show Desktop Still Hides Windows

The Windows Show Desktop command can minimize windows. Automatic restore is the recovery mechanism.

Turn on automatic restore for the profile, enable `Restore after Show Desktop`, and leave WindowAutoLayout running in the tray. The event guard restores already-running profile windows without launching apps or taking focus.

## Game Or Input Feels Affected

WindowAutoLayout does not register a global hotkey, raw mouse/keyboard device events, or low-level input hooks. Automatic restore is event-driven and has no recurring scan interval.

Check Task Manager for exactly one `WindowAutoLayout.exe` process. Its priority should be Below normal, and CPU should stay at or near zero while idle. Disable automatic restore temporarily to separate a shell-triggered restore from unrelated game behavior.

When the interface is closed, there should be no `msedgewebview2.exe` child owned by WindowAutoLayout. Reopening the interface creates WebView2 on demand; closing to tray removes it again.

Automatic restore is skipped whenever a known game or any fullscreen foreground window is active. If a restore was already waiting to run, returning to the game cancels it.

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
cargo test --all-targets
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
