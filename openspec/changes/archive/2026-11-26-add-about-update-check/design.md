# Design

## Context

See `proposal.md` for motivation and scope. Relevant codebase constraints:
- `desktop/src/app/settings_ui.rs` (`draw_about_tab`) renders the About card with logo, version (`CARGO_PKG_VERSION`), author, license, and repository hyperlink.
- `desktop/Cargo.toml` already depends on `ureq = "3"` with `default-features = false, features = ["json"]` for plain HTTP calls to the local `127.0.0.1:17890` Office bridge (`desktop/src/runtime.rs`). No TLS backend is currently enabled.
- `desktop/src/main.rs` enforces `#![forbid(unsafe_code)]`, and the workspace enforces `deny.toml` (`cargo deny --all-features --workspace --locked check advisories licenses bans sources`).
- `.github/workflows/release.yml` currently sets `prerelease: true` on `softprops/action-gh-release@v2`, while `Cargo.toml` and `scripts/package_native.py` publish clean `X.Y.Z` versions and multiple platform artifacts (`EqnTide-<ver>-Setup.exe`, `.zip`, `.deb`, `.rpm`, `.tar.gz`).
- `README.md` documents that EqnTide performs no background cloud calls or telemetry.

## Goals / Non-Goals

**Goals:**
- Keep the desktop app 100% offline/quiet until the user explicitly clicks **Check for updates** in the **About** tab.
- Re-use the existing `ureq` + `serde_json` + `EventSender<Event>` background thread pattern rather than introducing an async runtime (`tokio`) or heavy updater framework (`self_update`).
- Keep update delivery simple and safe across Windows (NSIS Setup + ZIP) and Linux (`.deb`, `.rpm`, `.tar.gz`) by delegating download/installation to the user's browser and existing OS installer/package manager.
- Align `.github/workflows/release.yml` with stable-only update checking while still allowing hyphenated `-beta`/`-rc` tags to publish as pre-releases.

**Non-Goals:**
- Automatic startup or periodic background checks.
- Modal update popups or status-bar nag badges.
- In-app binary downloading, SHA-256 self-patching, or Windows/Linux self-replacement (already handled cleanly by `EqnTide-<ver>-Setup.exe` or `apt`/`dnf`).
- Pre-release update channel support.

## Decisions

### 1. HTTP + TLS via `ureq` with `rustls`
- **Choice**: Enable `rustls` on `ureq` in `desktop/Cargo.toml` (e.g., `features = ["json", "rustls"]`) and add a helper in `desktop/src/runtime.rs` (or a small `desktop/src/update.rs` module) with a bounded timeout (e.g., 10s), explicit `User-Agent: eqntide-desktop/<version>`, and `Accept: application/vnd.github+json`.
- **Alternatives considered**:
  - `native-tls` / OpenSSL: introduces system OpenSSL linking pain on Linux builds (`build-native.yml`) and `ring`/`aws-lc-rs` vs `openssl-sys` headaches. `ureq` 3's default `rustls` feature is pure/portable across Windows and Linux (`build-native.yml` needs no extra `libssl-dev`).
  - Spawning `curl` / PowerShell: fragile error handling, slower, and OS-inconsistent.
- **Policy check**: Run `cargo deny --all-features --workspace --locked check advisories licenses bans sources` during implementation to confirm `rustls` transitive crates pass `deny.toml` cleanly.

### 2. GitHub endpoint & SemVer parsing
- **Choice**: Query `https://api.github.com/repos/vananh2801/eqntide/releases/latest`.
  - GitHub's `/releases/latest` returns the single latest release that is neither `draft` nor `prerelease`.
  - Parse `tag_name` by stripping an optional leading `v`/`V`, rejecting any tag containing `-` (prerelease) or `+` build issues (strip `+...` metadata if ever present, or require strict `major.minor.patch` digits), and comparing `(u64, u64, u64)` against `env!("CARGO_PKG_VERSION")`.
- **Alternatives considered**:
  - Adding a `semver` crate: unnecessary extra dependency for comparing 3 integers (`major.minor.patch`) when pre-releases are explicitly excluded.
  - Querying `/releases` list: larger JSON payload and unnecessary once `release.yml` publishes stable releases.

### 3. Platform-aware URL selection & validation
- **Choice**:
  - Extract `html_url` and `assets[]` (`name`, `browser_download_url`) from the release JSON.
  - Validate candidate URLs strictly: must begin with `https://github.com/vananh2801/eqntide/` (for `html_url` and `browser_download_url` release downloads). Reject any non-matching or non-HTTPS URL.
  - On Windows (`cfg!(target_os = "windows")`): prefer the asset whose `name` ends with `-Setup.exe` (matching `EqnTide-<ver>-Setup.exe` produced by `installer.nsi`), falling back to `html_url`.
  - On Linux / other OS: use `html_url` so the user picks `.deb`, `.rpm`, or `.tar.gz` appropriate for their distro.
  - Open the URL using `ctx.open_url(egui::OpenUrl::new_tab(url))`.
- **Alternatives considered**:
  - Always opening `html_url` on Windows too: works, but pointing Windows users directly at `-Setup.exe` (with `html_url` fallback + visible release page link) saves a click while still letting the user run the trusted NSIS installer.

### 4. App state & UI integration
- **Choice**:
  - Add an `UpdateState` enum on `App` (`Idle`, `Checking`, `UpToDate`, `Available { version, release_url, download_url }`, `Error(String)`) plus `Event::UpdateCheck(u64, Result<UpdateCheckOutcome, String>)` with a monotonic sequence counter `update_check_seq` so overlapping or stale responses are ignored.
  - Render an **Updates** section inside the **About** card (below the metadata `egui::Grid`, separated cleanly with spacing/separator) using existing `Self::standard_button`, `Self::primary_button_with`, `tokens`, and Tabler icons (no new icon rendering pipeline needed unless desired).
  - Do not persist `UpdateState` in `settings.json` (`desktop-settings-persistence` remains unchanged).

### 5. Release workflow change
- **Choice**: Change `prerelease: true` to `prerelease: ${{ contains(github.ref_name, '-') }}` in `.github/workflows/release.yml`, so clean `vX.Y.Z` tags publish as stable (`prerelease: false`) while optional hyphenated tags (e.g. `v0.12.0-beta.1`) stay `prerelease: true` without touching workflow YAML or creating extra branches. Keep package names (`eqntide-native-preview` in `package_native.py`) unchanged so existing `.deb`/`.rpm` users upgrade cleanly without package-rename conflicts.

## Risks / Trade-offs

- **GitHub unauthenticated API rate limit (60 req/hour/IP)** → Mitigation: manual button trigger only (not automatic on startup/tab switch), disable button while checking, and surface HTTP 403/429 cleanly as an inline retryable message (optionally linking to the repository Releases page).
- **`rustls` transitive dependencies vs `cargo-deny`** → Mitigation: verify `cargo deny --all-features --workspace --locked check` immediately after enabling `ureq`'s `rustls` feature; if `aws-lc-rs` or `ring` causes license/build friction on CI, evaluate `ureq` TLS provider features (`rustls` uses `ring` by default in `ureq` 3, which is MIT/ISC/OpenSSL-compatible; verify exact `deny.toml` license allowlist).
- **Existing `v0.11.0` release on GitHub is marked `prerelease: true`** → Mitigation: until the next tag (`v0.11.1` or `v0.12.0`) is published with the updated `release.yml`, `/releases/latest` will return 404 for `v0.11.0`; handle 404 clearly (e.g., treat "no stable release found yet" gracefully or note it in release notes/testing) and test with unit tests using mock JSON payloads / local mock HTTP server (`tiny_http` is already in root `Cargo.toml` or `std::net::TcpListener`).
