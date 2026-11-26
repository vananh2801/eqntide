# Tasks

## 1. Build-Time Pre-Release Version Injection

- [x] 1.1 Update `desktop/build.rs` to capture `EQNTIDE_BUILD_TAG_RAW` from `EQNTIDE_BUILD_TAG`, `GITHUB_REF_NAME`, or `git describe --tags --exact-match HEAD` (with `rerun-if-env-changed` / `.git` rerun directives and safe fallback), and verify `EQNTIDE_BUILD_TAG=v0.12.0-beta.1 cargo check -p eqntide-desktop` compiles cleanly.
- [x] 1.2 Add `desktop/src/version.rs` (`app_version()` and `resolve_app_version(...)`) with unit tests covering valid pre-release tags (`v0.12.0-beta.1`), mismatched tags (`v0.11.0-beta.1`), non-version refs (`main`), malformed suffixes, and fallback behavior, and verify `cargo test -p eqntide-desktop` passes.
- [x] 1.3 Verify `.github/workflows/build-native.yml` passes tag metadata (`GITHUB_REF_NAME`) cleanly into `cargo build` and smoke-checks `./target/.../eqntide-desktop --version`.

## 2. Desktop About UI & `--version` Display

- [x] 2.1 Update `desktop/src/app/settings_ui.rs` (About heading and up-to-date status string), `desktop/src/main.rs` (`--version` output), and existing About tests (`desktop/src/app/tests/layout_theme_tests.rs`, `desktop/src/app/tests/update_tests.rs`) to use `crate::version::app_version()`, and verify `EQNTIDE_BUILD_TAG=v0.12.0-beta.1 cargo run -p eqntide-desktop -- --version` prints `eqntide-desktop 0.12.0-beta.1`.

## 3. Pre-Release-Aware Update Checker

- [x] 3.1 Update `desktop/src/update.rs` (`spawn_update_check` and `evaluate_release_payload`) to pass `crate::version::app_version()`, parse running pre-release versions (`X.Y.Z-<suffix>`), and treat stable `vX.Y.Z` as newer than running `X.Y.Z-<suffix>` while keeping remote tag parsing strictly stable-only.
- [x] 3.2 Add unit tests in `desktop/src/update.rs` for `0.12.0-beta.1` vs. remote `v0.12.0` (`Available`), `0.12.0-beta.1` vs. remote `v0.11.0` (`UpToDate`), `0.12.0` vs. remote `v0.12.0` (`UpToDate`), and `0.12.0-beta.1` vs. remote `v0.12.0-rc.1` (still ignored), and verify `cargo test -p eqntide-desktop` passes.

## 4. Workspace Verification

- [x] 4.1 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, and verify all pass with zero warnings.
