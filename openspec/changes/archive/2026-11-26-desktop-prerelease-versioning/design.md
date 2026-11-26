# Design

## Context

`Cargo.toml`, `desktop/Cargo.toml`, and `crates/eqntide-editor/Cargo.toml` must keep clean `X.Y.Z` versions (enforced by `scripts/package_native.py` and `.deb`/`.rpm` packaging rules), while pre-release qualifiers such as `-beta.1` live on the Git tag (`v0.12.0-beta.1`) and control `.github/workflows/release.yml` (`prerelease: true`). See `proposal.md` for motivation.

Currently:
- `desktop/src/app/settings_ui.rs`, `desktop/src/main.rs`, and `desktop/src/update.rs` all embed `env!("CARGO_PKG_VERSION")`.
- `desktop/src/update.rs` uses `parse_stable_semver` for both the remote tag and `current_version`, rejecting any `-` suffix and preventing a `0.12.0-beta.1` build from seeing the later `v0.12.0` stable release as an update.

## Goals / Non-Goals

**Goals:**
- Capture the build-time tag candidate in `desktop/build.rs` into `EQNTIDE_BUILD_TAG_RAW` and resolve/validate the effective desktop display version via `desktop/src/version.rs` (`app_version()`) without modifying `Cargo.toml` versions.
- Display `app_version()` consistently in **About** (title and up-to-date status), `--version`, and the update checker's `User-Agent` / comparison input.
- Teach `desktop/src/update.rs` that `X.Y.Z-<prerelease>` is strictly older than `X.Y.Z` stable while keeping remote update targets strictly stable-only.

**Non-Goals:**
- Opting pre-release users into a separate beta update feed (update checks continue querying `/releases/latest` for stable releases only).
- Changing `.deb`/`.rpm`/NSIS/archive package filenames or `scripts/package_native.py` version validation.
- Changing `eqntide-cli --version` unless needed separately (scope is `eqntide-desktop`).

## Decisions

### 1. Build-time tag capture in `desktop/build.rs` + validation in `desktop/src/version.rs`
`desktop/build.rs` resolves the raw candidate tag string into `cargo:rustc-env=EQNTIDE_BUILD_TAG_RAW=...` using the following precedence order:
1. `EQNTIDE_BUILD_TAG` (explicit non-empty override env var, useful for tests or manual builds).
2. `GITHUB_REF_NAME` (automatically set by GitHub Actions on tag pushes like `v0.12.0-beta.1`).
3. `git describe --tags --exact-match HEAD` (when building locally directly on a checked-out tag).
4. Empty string `""` when none is available.

`desktop/src/version.rs` exposes `pub(crate) fn app_version() -> &'static str` (cached via `OnceLock<String>`) backed by `resolve_app_version(pkg_version, raw_tag)`:
- Strip an optional leading `v`/`V` from `raw_tag`.
- Accept the candidate only if its core `major.minor.patch` equals `pkg_version` and any `-<suffix>` contains only non-empty ASCII alphanumeric, `.`, or `-` characters (no leading/trailing `.`, no `..`, no whitespace).
- Otherwise ignore the candidate and return `pkg_version` (`CARGO_PKG_VERSION`).

*Alternatives considered:*
- *Putting `-beta.1` in `desktop/Cargo.toml` only:* Rejected because `scripts/package_native.py` (`validate_versions`) requires all three workspace manifests to match and strictly match `^\d+\.\d+\.\d+$`.
- *Validating only inside `desktop/build.rs`:* Rejected because `cargo test -p eqntide-desktop` does not execute `#[test]` functions inside `build.rs`; capturing the raw candidate in `build.rs` and validating in `desktop/src/version.rs` avoids duplicating parsing logic and makes all tag edge cases directly unit-testable under `cargo test`.

### 2. Rebuild invalidation in `desktop/build.rs`
Emit `cargo:rerun-if-env-changed=EQNTIDE_BUILD_TAG`, `cargo:rerun-if-env-changed=GITHUB_REF_NAME`, and `cargo:rerun-if-changed=../.git/HEAD` (plus `../.git/packed-refs` / `../.git/refs/tags` when those paths exist) so Cargo refreshes `EQNTIDE_BUILD_TAG_RAW` when tags or overrides change, without failing in source tarballs that have no `.git` directory.

### 3. SemVer ordering for running pre-release vs. remote stable in `desktop/src/update.rs`
Split version parsing into:
- `parse_stable_semver(raw)` — unchanged strict `(u64, u64, u64)` parser used for remote `tag_name` (remote pre-releases remain rejected).
- `parse_running_version(raw) -> Option<((u64, u64, u64), bool)>` — returns `(core_tuple, is_prerelease)` where `X.Y.Z` gives `is_prerelease = false` and `X.Y.Z-<non-empty>` gives `is_prerelease = true`.

Comparison rule against a validated stable remote `(r_maj, r_min, r_pat)`:
- Available if `remote_core > current_core`, OR `(remote_core == current_core && current_is_prerelease)`.
- Otherwise `UpToDate`.

*Alternatives considered:*
- *Full SemVer pre-release precedence between two pre-release tags:* Unnecessary because remote updates are stable-only (`/releases/latest`, non-prerelease).

### 4. UI presentation in About
`desktop/src/app/settings_ui.rs` replaces `env!("CARGO_PKG_VERSION")` with `crate::version::app_version()` in both the About heading (`EqnTide 0.12.0-beta.1`) and the up-to-date status message (`EqnTide 0.12.0-beta.1 is up to date.`), matching `desktop/src/main.rs` (`--version`). No layout restructure is required since the heading already lays out dynamically.

## Risks / Trade-offs

- **Local build after bumping/tagging without `git` or on a branch `HEAD` ahead of tag** → `git describe --tags --exact-match` only matches when `HEAD` is directly tagged, so normal dev commits safely fall back to `CARGO_PKG_VERSION` instead of showing an old tag.
- **Branch or PR `GITHUB_REF_NAME` in CI (e.g., `main`)** → The `CARGO_PKG_VERSION` core-match check rejects non-version refs like `main`, falling back cleanly to `CARGO_PKG_VERSION`.
- **Running `cargo check`/`clippy` when `desktop/build.rs` invokes `git`** → `Command::new("git")` must ignore missing `git` binary or non-zero exit status and never panic.
