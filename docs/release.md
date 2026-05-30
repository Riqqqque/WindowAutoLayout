# Release Notes

WindowAutoLayout ships through GitHub Releases. Tags are the release trigger, and the version source of truth is `package.json` plus the matching Tauri and Cargo version fields.

## Local Finish Pass

```powershell
npm run check
cd src-tauri
cargo fmt --check
cargo test
cd ..
npm run desktop:build
powershell -ExecutionPolicy Bypass -File scripts\install-current.ps1 -SkipBuild
```

The install helper verifies the uninstall entry, installed exe path, installer hash, and a real launch smoke.

## Publish

```powershell
git tag v0.1.2
git push origin main
git push origin v0.1.2
```

The GitHub workflow:

- validates the tag matches `package.json`
- runs TypeScript and Rust checks
- builds the Windows Tauri bundle
- generates `.sha256` files for installers
- uploads CI artifacts
- publishes tagged releases with the setup exe, MSI, and checksums
