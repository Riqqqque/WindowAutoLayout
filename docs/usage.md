# Usage Guide

WindowAutoLayout is built around one idea: save a workspace once, then bring it back with one action.

## Main Terms

| Term | Meaning |
| --- | --- |
| Profile | A saved workspace, such as Streaming or Editing |
| App entry | One app inside a profile, with matching and launch settings |
| Layout | The saved X, Y, width, and height for an app window |
| Target monitor | The monitor where the saved layout should land |
| Restore | The action that launches missing apps, finds windows, and moves them |
| Layout lock | An event-driven guard that recovers a profile after Show Desktop or leaving a fullscreen app |

## First Setup

1. Open the apps you want in the profile.
2. Open WindowAutoLayout.
3. Go to Settings and choose the default monitor if the profile should use one monitor by default.
4. Go to Profiles and pick the profile you want to edit.
5. Arrange the apps exactly how you want them.
6. Go to Dashboard, pick the monitor to capture, and press Capture current layout.
7. Fine-tune any app entry in Apps or any single window in Layout if needed.
8. Press Restore.

Once that works, enable startup restore or use the tray menu.

## Profiles

A profile groups apps together. A typical streaming profile might include:

- OBS Studio
- Discord
- Twitch or YouTube dashboard in a browser
- Stream music app
- Chat or moderation tool

Profiles can have their own target monitor. If an app entry also has a target monitor, the app entry wins for that app.

## App Entries

Each app entry controls how WindowAutoLayout finds, launches, and restores one app.

Important fields:

| Field | What To Use It For |
| --- | --- |
| Display name | Human-friendly name shown in the UI and logs |
| Executable path | Optional exact app path for launching a closed app |
| Process name | Best everyday matching rule, such as `obs64.exe` |
| Arguments | Optional command-line arguments used when launching |
| Working directory | Optional launch working folder |
| Title rule | Narrows matching when one process has many windows |
| Class name | Advanced matching when title/process is not enough |
| Window state | Normal, maximized, or minimized after restore |
| Detection timeout | How long restore waits for the window |
| Retry interval | How often restore checks while waiting |

For most apps, process name plus saved layout is enough.

## Saving Layouts

Use Layout when you want full control over which real window belongs to an app entry.

Use Dashboard capture when the visible windows on one monitor should become the current profile. Capture sets the profile target monitor, replaces that profile's app list with the visible windows on the selected monitor, and saves their monitor-relative positions.

Good save flow:

1. Put the target apps where you want them.
2. Confirm the monitor is correct.
3. Capture from Dashboard or save each app from Layout.
4. Move the windows somewhere else.
5. Restore the profile.
6. Check Logs if anything does not move.

## Restore Behavior

Restore works in this order:

1. Finds matching visible, minimized, or hidden windows according to the app settings.
2. Pulls hidden or minimized windows forward when allowed.
3. Starts missing apps when launching is enabled.
4. Waits for delayed main windows to appear.
5. Applies the saved layout.
6. Logs success, skip, launch, match, and error details.

If an app is running but no usable window appears, restore reports that instead of silently pretending it worked.

## OBS Setup

Recommended OBS app entry:

| Setting | Value |
| --- | --- |
| Process name | `obs64.exe` |
| Pull hidden/tray windows | On |
| Wake running tray apps | On |
| Restore if minimized | On |
| Detection timeout | At least 25 seconds |
| Retry interval | Around 700 ms |

OBS tray behavior is special. When OBS is hidden in the system tray, Windows may expose only helper windows or no normal main window. WindowAutoLayout asks OBS through its Qt tray icon message window before moving it, which matches the manual tray click path and gives OBS time to repaint.

## Discord Setup

Recommended Discord app entry:

| Setting | Value |
| --- | --- |
| Process name | `Discord.exe` |
| Pull hidden/tray windows | On |
| Wake running tray apps | On |
| Detection timeout | At least 25 seconds |

Discord can take a moment to expose its main Electron window after launch. Keep the timeout generous.

## Startup Restore

Startup restore is controlled from Settings, with the default profile selected under Profiles.

Settings control whether WindowAutoLayout starts with Windows, whether it starts minimized to tray, how long it waits, and whether it restores on launch.

Profiles control which profile is the default startup profile.

The startup command is stored in the current user's Run key:

```text
"<installed WindowAutoLayout.exe>" --startup-restore
```

No admin rights are required for normal startup registration.

## Layout Lock

Layout lock is for recovering a workspace from shell actions without continuous background work.

When enabled:

- Show Desktop triggers a restore of already-running profile windows.
- Leaving a game or fullscreen app triggers a delayed restore.
- Returning to a game before restore begins cancels the background restore.
- Automatic restore never launches a missing app or forces keyboard focus.
- No window scan runs on a timer while the desktop is idle.

Use the Dashboard Restore button or tray Restore command when missing apps also need to launch.

## Config And Logs

User config and logs are stored under:

```text
%APPDATA%\com.rique.windowautolayout
```

The config is JSON and can be imported or exported from Settings. Imports are parsed and normalized by the Rust backend before the interface uses them. If the config becomes unreadable, WindowAutoLayout backs it up and starts from a fresh default config.

## Everyday Streaming Flow

1. Start the PC.
2. Let WindowAutoLayout start from Windows.
3. OBS can already be in the tray or closed.
4. WindowAutoLayout waits for the startup delay.
5. Missing apps open.
6. Tray apps are pulled forward.
7. The selected streaming profile is restored.
8. Layout lock recovers the workspace after relevant shell events if enabled.
