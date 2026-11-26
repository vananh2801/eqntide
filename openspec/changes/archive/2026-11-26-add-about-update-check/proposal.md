# Proposal

## Why

Users running the EqnTide desktop application currently have no in-app way to check whether a newer stable release has been published on GitHub, and `.github/workflows/release.yml` currently marks every tagged release with `prerelease: true`, preventing GitHub's `/releases/latest` endpoint from returning stable releases. Adding a manual update check in the About tab lets users verify their version on demand and jump directly to the official release/download page without introducing background network telemetry or risky self-modifying installers.

## What Changes

- Add a manual **Check for updates** control and inline status/action UI inside the desktop **About** tab (no startup auto-check, no popup dialogs, no automatic download or installation).
- Query GitHub's stable latest-release endpoint (`GET /repos/vananh2801/eqntide/releases/latest`) on a background thread when the user triggers a check, parsing stable `vX.Y.Z` tags and comparing SemVer versions against `CARGO_PKG_VERSION`.
- Present clear inline states in About (`Idle`, `Checking`, `Up to date`, `Update available` with release-note summary/action button to open the platform-appropriate download or release URL in the browser, and `Error` with retry).
- Enable TLS support in `desktop/Cargo.toml` for `ureq` (`rustls`) while keeping `cargo deny` compliance intact.
- Update `.github/workflows/release.yml` so tagged stable releases (`vX.Y.Z`) publish with `prerelease: false` while pre-release tags containing `-` (e.g. `v0.12.0-beta.1`) automatically publish with `prerelease: true` via `${{ contains(github.ref_name, '-') }}`.

## Capabilities

### New Capabilities
- `desktop-update-check`: On-demand stable release update checking in the desktop About view against GitHub Releases, SemVer comparison, and browser handoff for downloading updates.

### Modified Capabilities
<!-- No existing capability requirements change; existing UI theme, button sizing, and cargo-deny workspace policies continue to apply as-is. -->

## Impact

- **Desktop crate (`desktop/`)**: `Cargo.toml` (`ureq` TLS feature), `src/update.rs` (GitHub HTTPS request + release JSON/SemVer parser), `src/main.rs` (`Event` variant), `src/app/mod.rs` / `src/app/events.rs` / `src/app/settings_ui.rs` (About UI state, background trigger, inline update card/row).
- **CI/Release workflow**: `.github/workflows/release.yml` (`prerelease: ${{ contains(github.ref_name, '-') }}`).
- **Docs**: `README.md` (note manual on-demand GitHub update check in About while keeping zero background telemetry).
- **Dependencies & Policy**: `Cargo.lock` and `cargo deny --all-features --workspace --locked check advisories licenses bans sources` verification for the added TLS dependency tree.
