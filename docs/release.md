# Release Notes

WindowAutoLayout ships through GitHub Releases. Tags are the release trigger, and the version source of truth is `package.json` plus the matching Tauri and Cargo version fields.

## Version Files

Update these together for every release:

- `package.json`
- `package-lock.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src/lib/api.ts`
- `examples/windowautolayout.example.json`
- `CHANGELOG.md`

The tag must match the package version:

```text
package.json version 0.1.28 -> tag v0.1.28
```

## Local Finish Pass

Run the checks and build locally:

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
```

Run a locked Rust advisory scan with a project-local `cargo-audit` binary when available. Warnings for dependencies that are not in the Windows target graph should be documented separately from actual vulnerabilities.

Install or update this PC from the freshly built bundle:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\install-current.ps1 -SkipBuild
```

The install helper verifies:

- installer path
- installer SHA256
- Windows uninstall entry
- installed exe path
- installed app version
- launch smoke, unless `-NoLaunchSmoke` is used

After the smoke launch, confirm tray-only mode has no WindowAutoLayout-owned WebView2 child, sample idle CPU, reopen and close the interface once, and verify the native process remains available from the tray.

## Manual Smoke

Before publishing a release, run the manual cases in [manual-test.md](manual-test.md). For OBS-related changes, do not skip the installed-app OBS tray test.

Minimum OBS release check:

1. Leave OBS running in the system tray.
2. Stop any running WindowAutoLayout process.
3. Launch the installed `WindowAutoLayout.exe --startup-restore`.
4. Confirm OBS stays one process.
5. Confirm no `OBS is already running` prompt appears.
6. Confirm the main OBS window is visible and not white.
7. Confirm Logs show restore success.

## Publish

Before creating a public tag:

- replace the pending `LICENSE` text with the final distribution terms
- configure Authenticode signing for both Windows installers
- verify the signing certificate and timestamp service in CI
- add a signed update channel before advertising automatic updates

The workflow intentionally rejects release tags while `LICENSE` still contains the pending no-redistribution text. Do not publish unsigned installers as a production release.

Commit the release changes, then push main and the version tag:

```powershell
git push origin main
git tag v0.1.28
git push origin v0.1.28
```

The GitHub workflow:

- validates the tag matches `package.json`
- blocks distribution while the repository license is still pending
- runs TypeScript checks
- runs the frontend dependency audit
- runs Rust format, check, test, and clippy passes
- builds the Windows Tauri bundle
- generates `.sha256` files for installers
- uploads CI artifacts
- publishes tagged releases with the setup exe, MSI, and checksums

## Release Assets

Expected release assets:

```text
WindowAutoLayout_<version>_x64-setup.exe
WindowAutoLayout_<version>_x64-setup.exe.sha256
WindowAutoLayout_<version>_x64_en-US.msi
WindowAutoLayout_<version>_x64_en-US.msi.sha256
```

Verify the published release:

```powershell
gh release view v0.1.28 --repo Riqqqque/WindowAutoLayout --json url,tagName,name,isDraft,isPrerelease,assets
```

## Troubleshooting Release Builds

If the workflow fails before build:

- confirm the tag matches `package.json`
- confirm `package-lock.json` is committed
- confirm Rust tests pass locally

If the workflow builds but release assets are missing:

- check the `src-tauri/target/release/bundle` paths in the workflow
- confirm the `.exe` and `.msi` files were produced
- confirm the publish step has `contents: write`

If the local installer looks stale:

- rebuild with `npm run desktop:build`
- run `scripts\install-current.ps1 -SkipBuild`
- check the installed exe file version under `%LOCALAPPDATA%\WindowAutoLayout`
