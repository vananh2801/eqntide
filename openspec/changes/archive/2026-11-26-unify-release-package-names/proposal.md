# Proposal

## Why

GitHub Release `v1.0.0` currently publishes inconsistently named assets across platforms and package formats (`EqnTide-1.0.0-Setup.exe`, `eqntide-native-preview-1.0.0-x86_64-pc-windows-msvc.zip`, `eqntide-native-preview-1.0.0-x86_64-unknown-linux-gnu.tar.gz`, `eqntide-native-preview_1.0.0_amd64.deb`, `eqntide-native-preview-1.0.0-1.x86_64.rpm`, and `README-WINDOWS.txt`), mixing legacy `native-preview` branding, inconsistent casing, Rust target triples, `amd64`/`x86_64` labels, and an unwanted Windows portable archive. Additionally, pre-release tags such as `v1.0.0-beta.1` appear inside `eqntide-desktop` but are omitted from packaged asset filenames.

## What Changes

- Unify all release artifact filenames under a lowercase `eqntide-<release-version>-<os>-<arch>` convention using `x64` for 64-bit x86 builds:
  - Windows installer: `eqntide-<release-version>-windows-x64-setup.exe`
  - Linux portable tarball: `eqntide-<release-version>-linux-x64.tar.gz`
  - Linux DEB package: `eqntide-<release-version>-linux-x64.deb`
  - Linux RPM package: `eqntide-<release-version>-linux-x64.rpm`
- Resolve `<release-version>` from matching build/git tags (`EQNTIDE_BUILD_TAG`, `GITHUB_REF_NAME`, or `git describe --tags --exact-match HEAD`) so pre-release tags like `v1.0.0-beta.1` appear in packaged filenames while keeping `Cargo.toml` at clean `X.Y.Z` and maintaining valid DEB/RPM internal package versions.
- **BREAKING**: Remove the Windows portable `.zip` archive from packaging outputs and release assets, and remove `README-WINDOWS.txt` / `docs/WINDOWS-INSTALLATION.txt` from the repository and release assets.
- Rename Linux DEB/RPM package identity and desktop entry from `eqntide-native-preview` (`EqnTide Native Preview`) to `eqntide` (`EqnTide`), adding upgrade/replacement metadata (`Conflicts`/`Replaces`/`Provides`/`Obsoletes`) for `eqntide-native-preview` alongside `eqn-tide`.
- Update the Windows desktop update checker (`desktop/src/update.rs`) to match `-setup.exe` instead of `-Setup.exe`.

## Capabilities

### New Capabilities
- `release-packaging`: Unified cross-platform release artifact naming, tag-aware pre-release filename versioning, Windows installer-only packaging, and Linux package upgrade continuity.

### Modified Capabilities
- `desktop-update-check`: Update the Windows installer asset matching rule from `-Setup.exe` to `-setup.exe`.

## Impact

- Packaging and installer scripts: `scripts/package_native.py`, `scripts/installer.nsi`
- CI and release workflows: `.github/workflows/build-native.yml`, `.github/workflows/release.yml`
- Desktop update checker and tests: `desktop/src/update.rs`, `desktop/src/app/tests/update_tests.rs`
- Packaging integration tests: `tests/python/test_native_integration.py`
- Documentation and repo skills: `README.md`, `docs/WINDOWS-INSTALLATION.txt` (removed), `.agents/skills/eqntide-release/SKILL.md`
