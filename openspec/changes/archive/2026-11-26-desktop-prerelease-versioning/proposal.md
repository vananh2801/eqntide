# Proposal

## Why

EqnTide keeps `Cargo.toml` versions at clean `X.Y.Z` (for example `0.12.0`) and encodes pre-release qualifiers such as `-beta.1` solely on Git tags (`v0.12.0-beta.1`) to satisfy `.deb`/`.rpm`/NSIS packaging rules. Because `eqntide-desktop` hardcodes `env!("CARGO_PKG_VERSION")`, pre-release builds display `EqnTide 0.12.0` instead of `0.12.0-beta.1` in **About** (and `--version`), and the update checker treats `0.12.0-beta.1` as already equal to the future stable `v0.12.0` release instead of offering the upgrade.

## What Changes

- Inject an optional build-time release tag / display version (such as `0.12.0-beta.1`) into `eqntide-desktop` via `desktop/build.rs` and `.github/workflows/build-native.yml` while keeping all `Cargo.toml` manifests at clean `X.Y.Z`.
- Display the effective application version (including any pre-release suffix) in the desktop **About** header, update-status text, and `--version` CLI output.
- Update the desktop update checker (`desktop/src/update.rs`) to parse an optional pre-release suffix on the running application version so a pre-release build (e.g. `0.12.0-beta.1`) recognizes the matching stable release (`v0.12.0`) as an available update while continuing to ignore remote draft/pre-release tags.

## Capabilities

### New Capabilities

### Modified Capabilities
- `desktop-update-check`: Expose build-time pre-release version identity in the About view and compare running pre-release versions (e.g. `0.12.0-beta.1`) as older than the corresponding stable GitHub release (`v0.12.0`).

## Impact

- `desktop/build.rs`: Resolve build-time version metadata from CI/override environment variables or exact Git tag and emit compile-time `rustc-env` values with safe fallback to `CARGO_PKG_VERSION`.
- `desktop/src/app/settings_ui.rs` & `desktop/src/main.rs`: Render the effective display version in About and `--version`.
- `desktop/src/update.rs`: Extend current-version parsing and SemVer comparison to distinguish pre-release running builds from stable remote releases.
- `.github/workflows/build-native.yml` (if needed): Ensure tag metadata is available during release builds.
