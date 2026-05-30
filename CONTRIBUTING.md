# Contributing

WindowAutoLayout targets Windows first. Keep changes focused on reliability, low idle overhead, and clear failure reporting.

## Local Checks

Run these before opening a pull request:

```powershell
npm run check
cd src-tauri
cargo fmt --check
cargo test
cd ..
npm run desktop:build
```

## Native Code

- Keep Win32 calls inside the Rust backend.
- Avoid permanent polling loops.
- Do not add admin requirements for normal restore workflows.
- Return clear errors to the UI instead of panicking.

## UI

- Keep controls dense and predictable.
- Do not hide important restore failures.
- Make profile and app edits survive restart through the JSON config.
