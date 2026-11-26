# Design

## Context

See [proposal.md](proposal.md) for motivation and [specs/release-packaging/spec.md](specs/release-packaging/spec.md) and [specs/desktop-update-check/spec.md](specs/desktop-update-check/spec.md) for behavioral contracts.

Currently:
- `scripts/package_native.py` reads `VERSION` exclusively from `Cargo.toml` (`X.Y.Z`), builds `eqntide-native-preview-<ver>-<rust_triple>.{zip,tar.gz}`, `eqntide-native-preview_<ver>_amd64.deb`, and `eqntide-native-preview-<ver>-1.x86_64.rpm`, stages `windows-stage/` + `installer-config.nsh`, and copies `docs/WINDOWS-INSTALLATION.txt` to `README-WINDOWS.txt`.
- `scripts/installer.nsi` writes `EqnTide-${APP_VERSION}-Setup.exe` from `windows-stage/`.
- `desktop/build.rs` and `desktop/src/version.rs` already resolve pre-release suffixes from `EQNTIDE_BUILD_TAG`, `GITHUB_REF_NAME`, or `git describe --tags --exact-match HEAD` when the tag's `X.Y.Z` base matches `CARGO_PKG_VERSION`.
- `desktop/src/update.rs` selects Windows installer assets matching `.ends_with("-Setup.exe")`.

## Goals / Non-Goals

**Goals:**
- Emit uniform lowercase asset filenames (`eqntide-<release_version>-<os>-x64...`) across Windows and Linux.
- Surface matching pre-release tag suffixes (e.g. `1.0.1-beta.1`) in release filenames, `dist/<platform>/<release_version>/` output directories, and Windows installer metadata without modifying clean `X.Y.Z` crate manifests.
- Eliminate the Windows portable `.zip` and `README-WINDOWS.txt` / `docs/WINDOWS-INSTALLATION.txt`.
- Rename Linux DEB/RPM package identity to `eqntide` while preserving clean upgrades from `eqntide-native-preview` and `eqn-tide`.
- Align `desktop/src/update.rs` with lowercase `-setup.exe`.

**Non-Goals:**
- Supporting 32-bit x86 (`i686`), ARM64 (`aarch64`), or macOS targets in CI or packaging (architecture mapping can map `aarch64` -> `arm64` defensively, but CI targets remain `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu`).
- Supporting case-insensitive or dual `-Setup.exe` / `-setup.exe` matching in `desktop/src/update.rs`.

## Decisions

### 1. Unified filename scheme and OS/arch mapping
All release assets follow `eqntide-<release_version>-<os>-<arch>`:
- Windows installer: `eqntide-<release_version>-windows-x64-setup.exe`
- Linux portable: `eqntide-<release_version>-linux-x64.tar.gz`
- Linux DEB: `eqntide-<release_version>-linux-x64.deb`
- Linux RPM: `eqntide-<release_version>-linux-x64.rpm`

In `scripts/package_native.py`, `arch_label` maps `x86_64` target prefixes to `x64` (and `aarch64` to `arm64`). `dpkg-deb` receives the explicit output path `eqntide-<release_version>-linux-<arch_label>.deb` while keeping `Architecture: amd64` (or `arm64`) inside `DEBIAN/control`. `rpmbuild` builds into its temporary `RPMS/` tree and the copied `.rpm` is renamed to `eqntide-<release_version>-linux-<arch_label>.rpm`.

*Alternative considered:* Keeping Debian/RPM filename conventions (`eqntide_1.0.0_amd64.deb`, `eqntide-1.0.0-1.x86_64.rpm`). Rejected because `amd64` vs `x86_64` vs `x64` caused visual inconsistency on GitHub Releases.

### 2. Tag-aware release version resolution in `package_native.py`
Mirror `desktop/src/version.rs` (`resolve_app_version`):
1. Read `CRATE_VERSION` from `Cargo.toml` and validate all three workspace manifests match clean `X.Y.Z`.
2. Resolve candidate tag from `EQNTIDE_BUILD_TAG`, else `GITHUB_REF_NAME`, else `git describe --tags --exact-match HEAD`.
3. Strip leading `v`/`V`. If `<core>-<suffix>` has `core == CRATE_VERSION` and a valid SemVer pre-release suffix (`[0-9A-Za-z-]+` dot-separated identifiers), set `RELEASE_VERSION = "<core>-<suffix>"`; otherwise `RELEASE_VERSION = CRATE_VERSION`.
4. `output_directory()` and `--print-version` use `RELEASE_VERSION`, so pre-release builds write to `dist/<platform>/<X.Y.Z-beta.1>/` (without colliding with `dist/<platform>/<X.Y.Z>/`), and `.github/workflows/build-native.yml`'s `$version = python scripts/package_native.py --print-version` continues to resolve `PACKAGE_DIR` directly.

### 3. Valid internal DEB and RPM versions for pre-release tags
Package managers enforce strict version syntax and ordering:
- **DEB**: `DEBIAN/control` uses `Version: <X.Y.Z>~<suffix>` for pre-releases (and `X.Y.Z` for stable). In Debian version comparison, `~` sorts strictly lower than the base `X.Y.Z` release so `1.0.1` cleanly upgrades `1.0.1~beta.1`.
- **RPM**: `.spec` keeps `Version: <X.Y.Z>` (since `rpmbuild` forbids `-` in `Version:`) and sets `Release: 0.<sanitized_suffix>` for pre-releases (with `-` replaced by `.`) vs `Release: 1` for stable, ensuring `1.0.1-1` cleanly upgrades `1.0.1-0.beta.1`.

### 4. Windows installer-only staging
When `windows` is true in `scripts/package_native.py`:
- Skip creating any `.zip` archive and skip copying `docs/WINDOWS-INSTALLATION.txt`.
- Still populate `dist/windows/<release_version>/windows-stage/` and `installer-config.nsh` (`!define APP_VERSION "<release_version>"` and `!define APP_ARCH "<arch_label>"`) so `scripts/installer.nsi` writes `OutFile "${PACKAGE_DIR}\eqntide-${APP_VERSION}-windows-${APP_ARCH}-setup.exe"`.
- Remove `docs/WINDOWS-INSTALLATION.txt` from the repository and drop `*.zip` and `README-*.txt` upload patterns from `.github/workflows/build-native.yml` and `.github/workflows/release.yml`.

### 5. Linux package rename and upgrade continuity
Rename `Package: eqntide-native-preview` / `Name: eqntide-native-preview` to `eqntide`, update `.desktop` `Name=EqnTide`, and declare replacement of both legacy package names:
- DEB `control`: `Conflicts: eqn-tide, eqntide-native-preview`, `Replaces: eqn-tide, eqntide-native-preview`, `Provides: eqn-tide, eqntide-native-preview`.
- RPM `.spec`: `Provides: eqn-tide = %{version}-%{release}`, `Provides: eqntide-native-preview = %{version}-%{release}`, `Obsoletes: eqn-tide <= %{version}-%{release}`, `Obsoletes: eqntide-native-preview <= %{version}-%{release}` (using an explicit versioned `Obsoletes` or unversioned `Obsoletes: eqntide-native-preview` so even `1.0.0-1` is obsoleted by `1.0.1-0.beta.1`).

### 6. Update checker lowercase `-setup.exe` match
Update `select_windows_setup_url` in `desktop/src/update.rs` to match `name.ends_with("-setup.exe")`, and update test fixtures in `desktop/src/update.rs` and `desktop/src/app/tests/update_tests.rs`.

## Risks / Trade-offs

- **Existing `v1.0.0` Windows clients** -> `v1.0.0` looks for `-Setup.exe` and will fall back to opening the GitHub Release page (`html_url`) rather than the direct `.exe` URL. Accepted because `v1.0.0` was only just released and users can still download from the opened release page.
- **RPM `Obsoletes` version comparison** -> If a pre-release `1.0.0-beta.1` had `Release: 0.beta.1`, `Obsoletes: eqntide-native-preview <= %{version}-%{release}` would NOT obsolete an older `1.0.0-1` unless unversioned or `<` next version. Using unversioned `Obsoletes: eqntide-native-preview` (or `Obsoletes: eqntide-native-preview < %{version}-999`) guarantees replacement across both pre-release and stable builds.
- **Local staging directory cleanup on Windows** -> Keep `windows-stage/` inside `dist/windows/<release_version>/` for `makensis` only; CI artifact upload (`build-native.yml`) only collects `dist/*/*/*.exe` on Windows, so `windows-stage/` is never uploaded.
