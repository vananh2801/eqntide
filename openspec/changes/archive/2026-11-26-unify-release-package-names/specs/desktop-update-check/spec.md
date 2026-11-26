# Spec Delta

## MODIFIED Requirements

### Requirement: Inline About feedback and safe browser download link
The **About** tab SHALL display update check results inline (`Checking`, `Up to date`, `Update available`, or `Error`) and, when an update is available, provide an action button that opens a validated `https://` GitHub release or installer URL in the user's system browser.

#### Scenario: Windows update action prefers the NSIS Setup installer asset
- **WHEN** an update is available on Windows and the release payload includes an HTTPS asset URL ending in `-setup.exe` under `https://github.com/`
- **THEN** clicking the update download button opens that installer URL (or falls back to the validated release `html_url` if no matching setup asset exists)

#### Scenario: Linux update action opens the release page
- **WHEN** an update is available on Linux
- **THEN** clicking the update download button opens the validated release `html_url` on GitHub so the user can choose `.deb`, `.rpm`, or `.tar.gz`

#### Scenario: Untrusted or non-HTTPS URLs are rejected
- **WHEN** a release payload contains a `html_url` or asset `browser_download_url` that does not start with `https://github.com/`
- **THEN** the application MUST ignore that URL rather than passing it to the system URL handler

#### Scenario: Network, HTTP, or rate-limit errors fail gracefully inline
- **WHEN** the GitHub request fails due to offline state, timeout, non-2xx HTTP status (including 403/429 rate limit), or invalid JSON
- **THEN** the About view displays a concise inline error message and allows the user to retry without crashing or showing a modal dialog
