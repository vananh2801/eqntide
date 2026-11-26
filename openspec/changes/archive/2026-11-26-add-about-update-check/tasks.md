# Tasks

## 1. Dependencies & Release Workflow

- [x] 1.1 Enable TLS (`rustls`) for `ureq` in `desktop/Cargo.toml` (`features = ["json", "rustls"]`), update `Cargo.lock`, and verify `cargo deny --all-features --workspace --locked check advisories licenses bans sources` passes with zero errors.
- [x] 1.2 Update `.github/workflows/release.yml` to set `prerelease: ${{ contains(github.ref_name, '-') }}` on `softprops/action-gh-release@v2` and verify YAML syntax / workflow job inputs are valid.

## 2. GitHub Release Update Checker

- [x] 2.1 Implement SemVer stable-tag parsing (`vX.Y.Z` without `-` prerelease suffixes), release payload parsing (`tag_name`, `draft`, `prerelease`, `html_url`, `assets`), and strict `https://github.com/vananh2801/eqntide/` URL validation with Windows `-Setup.exe` asset preference in `desktop/src/runtime.rs` (or `desktop/src/update.rs`), verified by unit tests covering newer, equal, older, prerelease/draft, and untrusted-URL payloads.
- [x] 2.2 Implement the background HTTPS fetch helper against `https://api.github.com/repos/vananh2801/eqntide/releases/latest` (bounded timeout, `User-Agent`, `Accept: application/vnd.github+json`, clean error mapping for network/HTTP/403/429/404), verified by a local mock HTTP test and `cargo test -p eqntide-desktop`.

## 3. Desktop About UI & Event Wiring

- [x] 3.1 Add `Event::UpdateCheck` in `desktop/src/main.rs`, `UpdateState` + `update_check_seq` in `desktop/src/app/mod.rs`, and event handling in `desktop/src/app/events.rs`, verified by unit tests checking sequence deduplication and state transitions without startup network calls.
- [x] 3.2 Add the **Check for updates** button, inline status display (`Checking`, `Up to date`, `Update available` with browser open action, `Error` with retry), and full-width card integration inside `draw_about_tab` in `desktop/src/app/settings_ui.rs`, verified by UI/layout tests and `cargo test --workspace`.
- [x] 3.3 Update `README.md` privacy/desktop notes to document the manual on-demand GitHub update check in About (clarifying zero automatic/startup network activity) and verify formatting/links.

## 4. Verification

- [x] 4.1 Run `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo deny --all-features --workspace --locked check advisories licenses bans sources` and verify all checks pass cleanly.
