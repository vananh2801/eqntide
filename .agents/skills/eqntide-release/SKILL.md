---
name: eqntide-release
description: Manage EqnTide version bumping, pre-release vs. stable tag conventions, pre-flight checks, local packaging, and GitHub Release tagging. Use when the user asks to prepare a release or beta/rc build, bump crate versions, run local packaging scripts, or tag/publish a release.
---

# EqnTide Release & Packaging

Follow this workflow whenever preparing a stable release, a pre-release (`beta`/`rc`), a version bump, or local distribution packages.

## 1. Channel & Tag Rules (Never Edit Workflow YAML Per Release)

`.github/workflows/release.yml` automatically controls GitHub Release visibility from the pushed tag name via `prerelease: ${{ contains(github.ref_name, '-') }}`:

- **Stable release (`vX.Y.Z`, no hyphen):** Sets `prerelease: false`, updates GitHub's `/releases/latest`, and makes the release visible to `eqntide-desktop`'s **About -> Check for updates**.
- **Pre-release (`vX.Y.Z-beta.1`, `vX.Y.Z-rc.1`, contains `-`):** Automatically sets `prerelease: true`, keeping it off `/releases/latest`.

Never create a special release branch or toggle `prerelease` manually in `.github/workflows/release.yml`.

## 2. Synchronize Crate Versions (`X.Y.Z` Only)

Before packaging or tagging, verify or bump the exact same clean `X.Y.Z` version (no `-beta` or `+meta` suffix) across all three workspace manifests:

1. `Cargo.toml` (`[package].version`) — read by `scripts/package_native.py` and `scripts/installer.nsi` for archive/package/installer names.
2. `desktop/Cargo.toml` (`[package].version`) — compiled into `eqntide-desktop` as `CARGO_PKG_VERSION`, shown in **About**, and compared by `desktop/src/update.rs`.
3. `crates/eqntide-editor/Cargo.toml` (`[package].version`) — kept aligned with the workspace.

Rules:
- Never put `-beta.1` into any `Cargo.toml`: `scripts/package_native.py` (`validate_versions`) rejects mismatched manifests or non-`X.Y.Z` versions, `rpmbuild` rejects `-` in `Version:`, `.deb` sorts hyphenated pre-releases higher than `X.Y.Z`, and `desktop/src/update.rs` (`parse_stable_semver`) parses `CARGO_PKG_VERSION` as three numeric components.
- For a beta tag like `v0.12.0-beta.1`, keep `Cargo.toml` versions at clean `X.Y.Z` (e.g., `0.11.0` or `0.12.0`) and put `-beta.1` only on the Git tag.
- Update `Cargo.lock` if workspace versions changed (`cargo check --workspace` updates lockfile workspace entries cleanly).
- Ask the user before choosing a new `X.Y.Z` version or deciding between stable vs. beta/rc if unspecified.

## 3. Pre-Flight Verification Gate

Run these checks before committing a release bump or tagging:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo deny --all-features --workspace --locked check advisories licenses bans sources
python3 scripts/render_icons.py --check
python3 -m py_compile scripts/package_native.py tests/python/test_native_integration.py
python3 -m unittest discover -s tests/python -p "test_*.py"
node --check wps-addin/main.js
node --check wps-addin/taskpane.js
```

Also run `npm test --prefix tests/wps` when `tests/wps/node_modules` is present (or `tests/run_all.sh --quick`), and rebuild `word-template/EqnTide.dotm` via `python3 word-template/build_dotm.py` if `word-template/EqnTide.bas` changed.

## 4. Local Distribution Packaging

Always build the full `--workspace` (root `Cargo.toml` sets `default-members = ["."]` so omitting `--workspace` skips `eqntide-desktop`):

### Linux (`dist/linux/<version>/`)

```bash
cargo build --workspace --release --locked
python3 scripts/package_native.py --target x86_64-unknown-linux-gnu --format all
```

### Windows (`dist/windows/<version>/`)

```powershell
cargo build --workspace --release --locked --target x86_64-pc-windows-msvc
python scripts/package_native.py --target x86_64-pc-windows-msvc
$version = python scripts/package_native.py --print-version
makensis "/DPACKAGE_DIR=$((Resolve-Path "dist/windows/$version").Path)" scripts/installer.nsi
```

Verify Windows Setup output keeps the `-Setup.exe` suffix (`EqnTide-<version>-Setup.exe`), which `desktop/src/update.rs` selects for Windows browser download handoff.

## 5. Tagging & Publish Safety

Before creating or pushing a release tag:

1. Confirm `git status --short` is clean (all version bumps and doc updates committed).
2. Confirm `python3 scripts/package_native.py --print-version` returns the expected `X.Y.Z`.
3. Never run `git tag` or `git push origin <tag>` without explicit confirmation from the user stating the exact tag (`vX.Y.Z` or `vX.Y.Z-beta.1`) and confirming the resulting GitHub Release mode (`stable` vs `prerelease`).
