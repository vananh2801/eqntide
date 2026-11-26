# Spec Delta

## Purpose

Provides on-demand stable update checking inside the desktop About view by querying GitHub Releases, comparing SemVer versions, and opening the official release or installer link in the user's browser without background telemetry or self-modifying binaries.

## ADDED Requirements

### Requirement: On-demand update check triggered only from About
The desktop application SHALL check for updates only when the user explicitly clicks the **Check for updates** control in the **About** tab, and SHALL NOT perform automatic update checks at startup, on tab navigation, or via modal popups.

#### Scenario: Startup and tab navigation remain offline
- **WHEN** the user launches the desktop application or switches between **Editor**, **Batch DOCX**, **Settings**, and **About** without clicking **Check for updates**
- **THEN** the application MUST NOT initiate any outbound network request to GitHub

#### Scenario: Clicking Check for updates runs asynchronously without blocking UI
- **WHEN** the user clicks **Check for updates** in the **About** tab
- **THEN** the application starts the check on a background thread, transitions the About update status to a checking state, disables duplicate triggers while in flight, and keeps the GUI responsive

### Requirement: Stable-only GitHub Release evaluation and SemVer comparison
The update checker SHALL query the repository's latest stable GitHub Release (`https://api.github.com/repos/vananh2801/eqntide/releases/latest`), reject draft, prerelease, or pre-release-suffixed tags (`X.Y.Z-...`), and compare parsed `(major, minor, patch)` SemVer tuples against the running `CARGO_PKG_VERSION`.

#### Scenario: Newer stable release is detected
- **WHEN** GitHub returns a non-draft, non-prerelease tag such as `v0.12.0` whose `(major, minor, patch)` version is strictly greater than `CARGO_PKG_VERSION`
- **THEN** the About view reports an available update with the remote version tag

#### Scenario: Running version matches or exceeds latest stable release
- **WHEN** GitHub returns a stable tag equal to or older than `CARGO_PKG_VERSION`
- **THEN** the About view reports that the installed version is up to date

#### Scenario: Prerelease or malformed release tags are not offered as updates
- **WHEN** a release payload is marked `draft` or `prerelease`, or has a non-stable or unparseable tag (e.g. `v0.12.0-rc.1`)
- **THEN** the update checker MUST NOT treat it as an available stable update

### Requirement: Inline About feedback and safe browser download link
The **About** tab SHALL display update check results inline (`Checking`, `Up to date`, `Update available`, or `Error`) and, when an update is available, provide an action button that opens a validated `https://` GitHub release or installer URL in the user's system browser.

#### Scenario: Windows update action prefers the NSIS Setup installer asset
- **WHEN** an update is available on Windows and the release payload includes an HTTPS asset URL ending in `-Setup.exe` under `https://github.com/`
- **THEN** clicking the update download button opens that installer URL (or falls back to the validated release `html_url` if no matching Setup asset exists)

#### Scenario: Linux update action opens the release page
- **WHEN** an update is available on Linux
- **THEN** clicking the update download button opens the validated release `html_url` on GitHub so the user can choose `.deb`, `.rpm`, or `.tar.gz`

#### Scenario: Untrusted or non-HTTPS URLs are rejected
- **WHEN** a release payload contains a `html_url` or asset `browser_download_url` that does not start with `https://github.com/`
- **THEN** the application MUST ignore that URL rather than passing it to the system URL handler

#### Scenario: Network, HTTP, or rate-limit errors fail gracefully inline
- **WHEN** the GitHub request fails due to offline state, timeout, non-2xx HTTP status (including 403/429 rate limit), or invalid JSON
- **THEN** the About view displays a concise inline error message and allows the user to retry without crashing or showing a modal dialog

### Requirement: Stable GitHub Release workflow publication
The release publication workflow SHALL publish clean `vX.Y.Z` tags as stable GitHub Releases (`prerelease: false`) so GitHub's `/releases/latest` endpoint resolves newly published stable releases, while marking hyphenated pre-release tags containing `-` (e.g. `v0.12.0-beta.1`) as `prerelease: true`.

#### Scenario: Stable tag is discoverable via latest-release endpoint
- **WHEN** `.github/workflows/release.yml` publishes a clean stable tag without `-` (e.g. `v0.12.0`)
- **THEN** `prerelease` evaluates to `false`, making the release visible to `/repos/vananh2801/eqntide/releases/latest`

#### Scenario: Pre-release tag stays marked as prerelease
- **WHEN** `.github/workflows/release.yml` publishes a tag containing `-` (e.g. `v0.12.0-beta.1`)
- **THEN** `prerelease` evaluates to `true`, preventing the pre-release from overwriting `/repos/vananh2801/eqntide/releases/latest`
