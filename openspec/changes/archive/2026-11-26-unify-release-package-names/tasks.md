# Tasks

## 1. Packaging Scripts & NSIS Installer

- [x] 1.1 Update `scripts/package_native.py` to resolve `RELEASE_VERSION` from `EQNTIDE_BUILD_TAG`, `GITHUB_REF_NAME`, or `git describe --tags --exact-match HEAD` (falling back to clean `CRATE_VERSION`), map `x86_64` targets to `x64`, emit `eqntide-<release_version>-linux-x64.{tar.gz,deb,rpm}`, use valid pre-release DEB/RPM internal versions (`~` / `Release: 0.<suffix>`), rename Linux package identity and desktop entry from `eqntide-native-preview` to `eqntide` with replacement headers for `eqntide-native-preview` and `eqn-tide`, and stage Windows without creating a `.zip` or `README-WINDOWS.txt`; verify with `python3 -m py_compile scripts/package_native.py` and updated unit tests in `tests/python/test_native_integration.py` (`python3 -m unittest discover -s tests/python -p "test_*.py"`).
- [x] 1.2 Update `scripts/installer.nsi` to read `APP_ARCH` from `installer-config.nsh` and set `OutFile "${PACKAGE_DIR}\eqntide-${APP_VERSION}-windows-${APP_ARCH}-setup.exe"`; verify `tests/python/test_native_integration.py::NativeIntegration::test_windows_installer_inputs_and_uri_target_exist` passes and `OutFile` matches the unified naming scheme.
- [x] 1.3 Delete `docs/WINDOWS-INSTALLATION.txt` from the repository and confirm no remaining references expect `README-WINDOWS.txt` or `WINDOWS-INSTALLATION.txt`.

## 2. Desktop Update Checker

- [x] 2.1 Update `select_windows_setup_url` in `desktop/src/update.rs` to match assets ending in `-setup.exe` instead of `-Setup.exe`, and update mock asset fixtures in `desktop/src/update.rs` and `desktop/src/app/tests/update_tests.rs`; verify with `cargo test -p eqntide-desktop --locked`.

## 3. Workflows & Documentation

- [x] 3.1 Update `.github/workflows/build-native.yml` and `.github/workflows/release.yml` to rename `eqntide-native-preview-*` / `Native preview release` and remove `*.zip` and `README-*.txt` artifact globs; verify workflow YAML parses cleanly and matches `scripts/package_native.py` outputs.
- [x] 3.2 Update `README.md` and `.agents/skills/eqntide-release/SKILL.md` to remove Windows portable `.zip` instructions and document the unified `eqntide-<version>-{windows-x64-setup.exe,linux-x64.tar.gz,linux-x64.deb,linux-x64.rpm}` release asset names (including pre-release tag filename behavior); verify with `python3 -m unittest discover -s tests/python -p "test_*.py"`.

## 4. Final Verification

- [x] 4.1 Run workspace verification gates (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --all-targets --locked`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, and `python3 -m unittest discover -s tests/python -p "test_*.py"`) and confirm zero warnings or failures.
