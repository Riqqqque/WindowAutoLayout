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
| Automatic restore | An event-driven guard that recovers a profile after Show Desktop or leaving a fullscreen app |

## First Setup

1. Open the apps you want in the profile.
2. Open WindowAutoLayout.
3. Go to Settings and choose the default monitor if the profile should use one monitor by default.
4. Go to Profiles and pick the profile you want to edit.
5. Arrange the apps exactly how you want them.
6. Go to Dashboard, pick the monitor to capture, and press Capture current layout.
7. Fine-tune any app entry in Apps or any single window in Layout if needed.
8. Press Restore windows now.

Once that works, enable startup restore or use the tray menu.

## Profiles

A profile groups apps together. A typical streaming profile might include:

- OBS Studio
- Discord
- Twitch or YouTube dashboard in a browser
- Stream music app
- Chat or moderation tool

Profiles can have their own target monitor. If an app entry also has a target monitor, the app entry wins for that app. New captures store a hardware-backed monitor identity, so Windows renumbering `DISPLAY1` and `DISPLAY2` does not silently retarget a saved workspace.

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

Use Dashboard capture when the visible windows on one monitor should become the current profile. Capture sets the profile target monitor, replaces that profile's app list with the visible windows on the selected monitor, and saves their monitor-relative physical-pixel positions. It also records the display resolution, usable work area, and scale so the layout can adapt if that display changes later.

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
4. Waits past small launch or updater splash windows for the real main surface.
5. Applies the saved layout.
6. Rechecks normal windows until the saved bounds hold across two geometry reads.
7. Logs success, skip, launch, match, and error details.

If an app is running but no usable window appears, restore reports that instead of silently pretending it worked.

`Launch apps that are closed` only controls new process launches. An existing minimized or tray-hidden app can still be recovered when that setting is off.

## Tray Controls

Right-click the tray icon to restore immediately, see whether automatic restore is on, open the app, open the activity log, or exit. The restore and automatic items are disabled while a restore is already running.

Settings controls what a normal left-click does:

- `Open WindowAutoLayout` shows the existing app window.
- `Restore windows now` triggers the default profile without opening the app.

The tray text, tooltip, and app header all use the same runtime status, so they cannot silently disagree about whether automatic restore is active.

Closing the visible interface to tray unloads its WebView processes. The native tray process stays resident and can still restore, toggle automatic recovery, or reopen the interface.

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

OBS tray behavior is special. When OBS is hidden in the system tray, Windows may expose only helper windows or no normal main window. WindowAutoLayout asks OBS through its Qt tray icon message window, waits for the main window, applies the saved rectangle, and verifies a real minimize/restore transition so Qt presents the rebuilt interface.

OpenLaunchDeck can start with Windows in `--background` mode. During a restore, WindowAutoLayout launches the saved OpenLaunchDeck executable with `--show`. OpenLaunchDeck hands that request to its existing single instance, restores the tray window, and exits the short-lived helper process.

Every restored window receives a non-activating, one-shot surface refresh after it is shown or moved. This asks toolkits such as Qt and Electron to redraw newly exposed client areas without synthetic input or background repaint polling. OBS uses the additional bounded recovery above and returns to the previous window when focus stayed on OBS during that recovery.

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

## Automatic Restore

Automatic restore is for recovering a workspace from shell actions without continuous background work.

When enabled:

- Show Desktop triggers a restore of already-running profile windows when its setting is enabled.
- Leaving a game or fullscreen app triggers a delayed restore when its setting is enabled.
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
8. Automatic restore recovers the workspace after relevant shell events if enabled.
