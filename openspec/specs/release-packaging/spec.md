# release-packaging Specification

## Purpose

Defines unified release asset naming, tag-aware pre-release filename versioning, Windows installer-only output, and Linux package identity and upgrade continuity for EqnTide native distributions.

## Requirements

### Requirement: Unified release artifact naming convention
The packaging pipeline SHALL emit release artifacts using lowercase `eqntide-<release-version>-<os>-<arch>` naming with `x64` as the architecture label for 64-bit x86 builds, and SHALL NOT emit legacy `native-preview` prefixes, Rust target triples, or `README-WINDOWS.txt`.

#### Scenario: Windows release asset uses unified installer filename
- **WHEN** packaging runs for a 64-bit Windows build at release version `1.0.1`
- **THEN** the emitted Windows installer is named `eqntide-1.0.1-windows-x64-setup.exe` and no Windows `.zip` portable archive or `README-WINDOWS.txt` is produced

#### Scenario: Linux release assets use unified x64 filenames
- **WHEN** packaging runs for a 64-bit Linux build (`--format all`) at release version `1.0.1`
- **THEN** the emitted artifacts are named `eqntide-1.0.1-linux-x64.tar.gz`, `eqntide-1.0.1-linux-x64.deb`, and `eqntide-1.0.1-linux-x64.rpm`

### Requirement: Tag-aware pre-release version in artifact filenames
When building or packaging from a release tag whose `X.Y.Z` base matches the workspace crate manifests and includes a valid pre-release suffix (such as `v1.0.1-beta.1`), the packaging pipeline SHALL embed the full `X.Y.Z-<suffix>` string in output filenames and the Windows installer display version while keeping `Cargo.toml` manifests at clean `X.Y.Z`.

#### Scenario: Pre-release tag populates asset filenames
- **WHEN** packaging runs with matching release tag `v1.0.1-beta.1` and crate version `1.0.1`
- **THEN** the emitted filenames use `1.0.1-beta.1` (for example `eqntide-1.0.1-beta.1-windows-x64-setup.exe` and `eqntide-1.0.1-beta.1-linux-x64.deb`)

#### Scenario: Untyped or mismatched tag falls back to crate version
- **WHEN** packaging runs without a tag or with a tag whose `X.Y.Z` base does not match the crate manifests
- **THEN** the emitted filenames use the clean crate `X.Y.Z` version

#### Scenario: Linux DEB and RPM internal metadata remain valid for pre-release builds
- **WHEN** `.deb` and `.rpm` packages are built for a pre-release tag such as `v1.0.1-beta.1`
- **THEN** internal DEB `Version` (`1.0.1~beta.1`) and RPM `Version`/`Release` (`1.0.1` / `0.beta.1`) stay valid for `dpkg-deb` and `rpmbuild` and sort strictly lower than the subsequent `1.0.1` stable package

### Requirement: Windows installer-only distribution
Windows distribution packaging SHALL stage the files required by the NSIS installer without creating a portable `.zip` archive or standalone `README-WINDOWS.txt` release asset.

#### Scenario: Windows packaging skips portable archive and readme
- **WHEN** the packaging script runs for a Windows target and NSIS builds the installer
- **THEN** only `eqntide-<release-version>-windows-x64-setup.exe` is produced as a distributable Windows release asset

### Requirement: Linux package identity and upgrade continuity
Linux `.deb` and `.rpm` packages and desktop entries SHALL use the `eqntide` package name and `EqnTide` desktop entry name, and SHALL declare upgrade/replacement relationships for prior `eqntide-native-preview` and `eqn-tide` packages.

#### Scenario: Upgrading from eqntide-native-preview replaces the legacy package cleanly
- **WHEN** a user installs `eqntide-<version>-linux-x64.deb` or `eqntide-<version>-linux-x64.rpm` on a system with `eqntide-native-preview` installed
- **THEN** the package metadata (`Conflicts`/`Replaces`/`Provides` in DEB; `Provides`/`Obsoletes` in RPM) replaces `eqntide-native-preview` cleanly without file ownership conflicts
