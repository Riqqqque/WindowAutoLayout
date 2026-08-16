# Contributing

WindowAutoLayout targets Windows first. Keep changes focused on reliable restore behavior, low idle overhead, and clear failure reporting.

## Priorities

- One restore action should bring the configured setup back.
- Tray apps should be handled without duplicate launches.
- OBS should remain smooth for Replay Buffer and streaming workflows.
- Normal restore workflows should not require admin rights.
- Failures should be visible in Logs instead of silently ignored.
- Background work should stay lightweight.

## Local Checks

Run these before publishing changes:

```powershell
npm run check
cd src-tauri
cargo fmt --check
cargo test
cd ..
npm run desktop:build
```

For release-quality changes, also install the built app and test the installed copy:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\install-current.ps1 -SkipBuild
```

## Native Code

- Keep Win32 calls inside the Rust backend.
- Avoid permanent busy loops.
- Prefer blocking Windows events over recurring polling for background behavior.
- Do not add global mouse hooks, keyboard hooks, raw-input listeners, or synthetic input.
- Treat hidden tray windows, minimized windows, and delayed windows as normal cases.
- Return clear errors to the UI instead of panicking.
- Keep matching conservative enough to avoid tool windows and helper windows.

## UI

- Keep controls dense and predictable.
- Prefer clear toggles, inputs, and selects over hidden behavior.
- Do not hide restore failures.
- Make profile and app edits survive restart through the JSON config.
- Keep labels short, direct, and close to the actual behavior.

## Docs

- Keep the README as the GitHub front door.
- Put deeper setup, troubleshooting, testing, and release details under `docs/`.
- Document behavior from the app as it actually works, especially for OBS, startup restore, and layout lock.
- Keep release notes plain and specific.

## Windows Validation

Important manual cases:

- OBS visible restore
- OBS minimized restore
- OBS hidden in tray restore
- closed app launch and restore
- Discord delayed window restore
- layout lock after Show Desktop
- startup restore from the installed exe
- bad executable path reporting
- corrupted config backup
