# Spec Delta

## ADDED Requirements

### Requirement: Build-time pre-release version identity in About and CLI
When the desktop application is built from a tagged pre-release matching its crate `X.Y.Z` version (such as `v0.12.0-beta.1`), the application SHALL display the full pre-release version string (e.g. `0.12.0-beta.1`) in the **About** header, **About** up-to-date status text, and `--version` output while keeping crate manifests at clean `X.Y.Z`.

#### Scenario: Pre-release tag build displays the pre-release suffix
- **WHEN** `eqntide-desktop` is built for release tag `v0.12.0-beta.1` with crate version `0.12.0`
- **THEN** the **About** view header shows `EqnTide 0.12.0-beta.1`, the up-to-date message references `EqnTide 0.12.0-beta.1`, and `eqntide-desktop --version` prints `eqntide-desktop 0.12.0-beta.1`

#### Scenario: Untyped or mismatched tag falls back to crate version
- **WHEN** `eqntide-desktop` is built without a matching release tag or when a tag's `X.Y.Z` base does not match the crate version
- **THEN** the application displays the clean crate `X.Y.Z` version without failing the build

## MODIFIED Requirements

### Requirement: Stable-only GitHub Release evaluation and SemVer comparison
The update checker SHALL query the repository's latest stable GitHub Release (`https://api.github.com/repos/vananh2801/eqntide/releases/latest`), reject draft, prerelease, or pre-release-suffixed remote tags (`X.Y.Z-...`), and compare the remote stable `(major, minor, patch)` version against the running application version including any pre-release qualifier.

#### Scenario: Newer stable release is detected
- **WHEN** GitHub returns a non-draft, non-prerelease tag such as `v0.12.0` whose `(major, minor, patch)` version is strictly greater than the running version
- **THEN** the About view reports an available update with the remote version tag

#### Scenario: Running version matches or exceeds latest stable release
- **WHEN** the running application is a stable build and GitHub returns a stable tag equal to or older than the running `(major, minor, patch)` version
- **THEN** the About view reports that the installed version is up to date

#### Scenario: Stable release matching a running pre-release is detected as an update
- **WHEN** the running application is a pre-release build such as `0.12.0-beta.1` and GitHub returns the non-draft, non-prerelease stable tag `v0.12.0`
- **THEN** the About view reports `v0.12.0` as an available update

#### Scenario: Running pre-release version exceeds latest stable release
- **WHEN** the running application is a pre-release build such as `0.12.0-beta.1` and GitHub returns an older stable tag such as `v0.11.0`
- **THEN** the About view reports that the installed version is up to date

#### Scenario: Prerelease or malformed release tags are not offered as updates
- **WHEN** a release payload is marked `draft` or `prerelease`, or has a non-stable or unparseable tag (e.g. `v0.12.0-rc.1`)
- **THEN** the update checker MUST NOT treat it as an available stable update
