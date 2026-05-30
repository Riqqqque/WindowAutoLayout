# Changelog

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
