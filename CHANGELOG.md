# Changelog

## 0.1.23

- Serialized restore requests so startup, tray, dashboard, and event-lock restores cannot move the same windows at the same time.
- Reported restores paused by a foreground game as paused instead of success, and kept the event guard ready to retry after the game exits.
- Preserved maximized state when capturing layouts and constrained oversized saved bounds after monitor resolution changes.
- Made nearest-monitor fallback compare the saved profile canvas against every connected display.
- Added safe config import parsing, schema normalization, duplicate-ID repair, regex validation, and bounds for imported delays and rectangles.
- Made a second launch bring the existing tray app forward and made the close-button setting exit cleanly when tray hiding is disabled.
- Removed settings that did not affect runtime behavior and made fullscreen game protection unconditional.
- Isolated page scrolling inside the app shell, reset scroll position between views, and added an interface recovery screen.

## 0.1.22

- Removed the global hotkey and disabled Tauri raw mouse and keyboard device events, so WindowAutoLayout no longer registers for background input.
- Replaced the five-second layout-lock poller with a blocking Windows event guard for Show Desktop and game-to-desktop transitions.
- Prevented automatic and startup restores from forcing window focus or synthesizing keyboard input.
- Added mid-restore game checks so background work stops if a game or fullscreen app becomes active.
- Coalesced restore events through one low-priority worker instead of creating a thread for every shell event.
- Tightened the Tauri content security policy and removed the unused opener plugin.
- Kept Microsoft Store app profiles working across versioned WindowsApps folder and executable-name changes.
- Redesigned the dashboard, layout editor, app editor, settings, profiles, and logs around a denser desktop control surface.

## 0.1.21

- Added an event-driven desktop guard that restores already-running profile windows after Show Desktop or after leaving a latency-sensitive game.
- Kept the guard idle while games such as Valorant are foreground and restored without launching missing apps.
- Left the installed app running after local installs instead of closing the smoke-launch process.
- Prevented duplicate tray processes with a named single-instance guard.
- Accepted BOM-marked config files so a valid config is not replaced with defaults.
- Kept startup registration pointed at `WindowAutoLayout.exe --startup-restore` so the tray app comes back on Windows login.

## 0.1.20

- Started the app hidden during startup restore so no WindowAutoLayout window is created on-screen before tray mode.
- Lowered WindowAutoLayout process priority and the layout-lock worker thread priority so the game scheduler wins.
- Raised layout-lock polling to a 5-30 second range and paused it for 60 seconds while Valorant or another latency-sensitive game is foreground.
- Treated tray-only OpenLaunchDeck as a clean no-op instead of relaunching it or marking the restore partial.
- Kept the exact-placement and low-CPU restore fixes from 0.1.19.

## 0.1.19

- Restored normal windows after unmaximizing before the final move, so Windows does not undo the saved bounds.
- Rechecked restored windows and retried the saved rectangle once if Windows reported drift after the move.
- Kept stale or off-screen saved layouts on the target monitor while preserving exact saved bounds when they still touch the monitor.
- Made Capture Current Layout use the window's real monitor, falling back to its center point only when Windows does not report a monitor.

## 0.1.18

- Cut layout lock CPU use by only running full restore work when managed windows actually change.
- Kept layout lock paused longer while a fullscreen non-profile app is foreground, so games do not get polled hard.
- Added tests around lock change detection and managed-window signatures.
- Updated the Windows build workflow to use the locked npm install, Rust format/check/test/clippy, and clean frontend dependency audit.

## 0.1.17

- Fixed restores getting blocked when Windows renumbers the saved monitor.
- Changed old default monitor-missing behavior to use the nearest available display instead of doing nothing.
- Scaled fallback layouts to the available monitor so saved workspaces do not land off-screen after display changes.

## 0.1.16

- Repaired startup registration on launch when saved settings say WindowAutoLayout should start with Windows.
- Tightened startup checks so the Run entry must point to the current app with the startup restore argument.

## 0.1.15

- Kept saved/imported layout lock settings in sync with the running lock thread.
- Made restore results say when windows were already in their saved positions instead of reporting a move.

## 0.1.14

- Made layout lock lighter for gaming by using a calmer default interval and pausing while a fullscreen foreground app is active.
- Stopped repeated lock passes from moving windows that are already in their saved position.

## 0.1.13

- Added delete buttons for profile restore items on Dashboard and Layout.
- Kept app removal consistent across the profile editor so selection moves cleanly to the next app.

## 0.1.12

- Added a capture current layout workflow that snapshots visible windows on a selected monitor into the active profile.
- Made capture set the profile target monitor and save window positions as monitor-relative layouts.
- Kept captured profiles ready to restore by saving process names, executable paths, class names, and safer title rules when needed.

## 0.1.11

- Refreshed the app shell with a cleaner dashboard, sidebar, controls, status panels, and layout editor.
- Added clearer lock/startup state and friendlier import errors for bad JSON.
- Tightened the monitor preview so stacked saved windows stay readable.

## 0.1.10

- Fixed OBS tray restore by asking OBS through its tray icon before moving the window, matching the manual click that makes OBS repaint correctly.
- Kept the restored OBS window activated and repainted through the final layout move.

## 0.1.9

- Fixed OBS tray restore so minimized-to-tray OBS is restored from its existing window instead of starting OBS again.
- Kept OBS restore waiting for the shown window to settle before applying the saved layout.

## 0.1.8

- Fixed OBS tray restore taking the hidden window shortcut before OBS had a chance to show itself.
- Made restore wait for a visible OBS window after waking the running process, then let OBS settle before applying the saved layout.

## 0.1.7

- Fixed OBS coming back as a white window after restore from the tray by waking and repainting the pulled window before layout finishes.
- Kept layout lock moves from repeatedly activating already-visible windows.

## 0.1.6

- Added a persistent layout lock toggle that keeps reapplying the selected profile while it is on.
- Remembered the locked profile and made the lock fast enough to bring windows back after Show Desktop or accidental minimize/move actions.

## 0.1.5

- Made restore always try to open missing apps in the selected profile before moving them.
- Simplified app setup so launch behavior is on by default instead of being a per-app trap.

## 0.1.4

- Let closed apps start from registry app paths, common install folders, PATH, and Start Menu shortcuts when no exact exe path is saved.
- Added a GitHub Desktop preset that can start the app before moving it.

## 0.1.3

- Fixed OBS restore picking detached dock panels like Stats instead of the real OBS main window.

## 0.1.2

- Let running tray apps wake even when "Launch if missing" is off, so OBS can be pulled back without needing a configured launch path.

## 0.1.1

- Added hidden/tray window pulling so apps like OBS can be restored from the tray before a new launch is attempted.
- Added a wake path for apps already running in the tray with no top-level window yet.
- Kept the detected window list aware of hidden and minimized windows for layout troubleshooting.

## 0.1.0

- First WindowAutoLayout build.
- Added monitor detection, window detection, layout capture, profile restore, app launching, startup registration, tray actions, temporary layout locking, logs, settings, and editable presets.
- Added Windows release workflow and manual test notes.
