//! On-demand GitHub Releases stable update checker for the desktop About view.

use crate::version::{app_version, parse_running_version, parse_stable_semver};
use crate::{Event, EventSender as Sender};
use std::time::Duration;

/// Public GitHub Releases page used as a safe fallback link.
pub(crate) const GITHUB_RELEASES_PAGE_URL: &str = "https://github.com/vananh2801/eqntide/releases";
const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/vananh2801/eqntide/releases/latest";
const ALLOWED_RELEASE_URL_PREFIX: &str = "https://github.com/vananh2801/eqntide/";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Metadata for a newer stable GitHub Release validated for browser handoff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateRelease {
    /// Normalized `major.minor.patch` version string without a leading `v`.
    pub version: String,
    /// Validated HTTPS URL to the GitHub Release page.
    pub release_url: String,
    /// Validated HTTPS URL to open when downloading (Windows installer or release page).
    pub download_url: String,
}

/// Result returned by a completed GitHub Release update check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateCheckOutcome {
    /// Installed version matches or exceeds the latest stable release.
    UpToDate,
    /// A strictly newer stable release is available on GitHub.
    Available(UpdateRelease),
}

/// UI state machine for the manual update checker in the About tab.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) enum UpdateState {
    /// No check has run in this session yet.
    #[default]
    Idle,
    /// An on-demand GitHub check is currently running in the background.
    Checking,
    /// Installed build is up to date with the latest stable GitHub Release.
    UpToDate,
    /// A newer stable release was found and is ready to open in the browser.
    Available(UpdateRelease),
    /// The most recent check failed with a human-readable error message.
    Error(String),
}

/// Spawns a background thread that checks GitHub for a newer stable release.
pub(crate) fn spawn_update_check(tx: Sender<Event>, seq: u64) {
    std::thread::spawn(move || {
        let result = check_latest_release(
            GITHUB_LATEST_RELEASE_URL,
            app_version(),
            cfg!(target_os = "windows"),
        );
        let _ = tx.send(Event::UpdateCheck(seq, result));
    });
}

/// Returns `true` when `url` is a clean HTTPS link under the EqnTide GitHub repository.
pub(crate) fn is_trusted_release_url(url: &str) -> bool {
    url.starts_with(ALLOWED_RELEASE_URL_PREFIX) && !url.chars().any(char::is_whitespace)
}

/// Fetches release JSON from `api_url` and evaluates whether a newer stable update exists.
fn check_latest_release(
    api_url: &str,
    current_version: &str,
    is_windows: bool,
) -> Result<UpdateCheckOutcome, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    let user_agent = format!("eqntide-desktop/{current_version}");
    let mut response = agent
        .get(api_url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", &user_agent)
        .call()
        .map_err(|err| format!("Could not reach GitHub: {err}"))?;

    let status = response.status().as_u16();
    match status {
        200 => {
            let payload: serde_json::Value = response
                .body_mut()
                .read_json()
                .map_err(|err| format!("Invalid GitHub release response: {err}"))?;
            evaluate_release_payload(&payload, current_version, is_windows)
        }
        403 | 429 => {
            Err("GitHub API rate limit reached. Try again later or open GitHub Releases.".into())
        }
        404 => Err("No published stable release found yet.".into()),
        other => Err(format!("Update check failed (HTTP {other}).")),
    }
}

/// Evaluates a GitHub Release JSON payload against `current_version`.
fn evaluate_release_payload(
    payload: &serde_json::Value,
    current_version: &str,
    is_windows: bool,
) -> Result<UpdateCheckOutcome, String> {
    if payload["draft"].as_bool() == Some(true) || payload["prerelease"].as_bool() == Some(true) {
        return Ok(UpdateCheckOutcome::UpToDate);
    }

    let raw_tag = payload["tag_name"]
        .as_str()
        .ok_or_else(|| "GitHub release response is missing tag_name.".to_string())?;
    if is_prerelease_tag(raw_tag) {
        return Ok(UpdateCheckOutcome::UpToDate);
    }

    let remote_semver = parse_stable_semver(raw_tag)
        .ok_or_else(|| format!("Unexpected release tag format: '{raw_tag}'."))?;
    let (current_semver, current_is_prerelease) = parse_running_version(current_version)
        .ok_or_else(|| format!("Invalid application version: '{current_version}'."))?;

    let is_newer_stable = remote_semver > current_semver
        || (remote_semver == current_semver && current_is_prerelease);
    if !is_newer_stable {
        return Ok(UpdateCheckOutcome::UpToDate);
    }

    let release_url = payload["html_url"]
        .as_str()
        .filter(|url| is_trusted_release_url(url))
        .ok_or_else(|| "GitHub release URL failed validation.".to_string())?
        .to_string();

    let download_url = if is_windows {
        select_windows_setup_url(payload).unwrap_or_else(|| release_url.clone())
    } else {
        release_url.clone()
    };

    Ok(UpdateCheckOutcome::Available(UpdateRelease {
        version: format!(
            "{}.{}.{}",
            remote_semver.0, remote_semver.1, remote_semver.2
        ),
        release_url,
        download_url,
    }))
}

fn is_prerelease_tag(raw: &str) -> bool {
    let trimmed = raw
        .trim()
        .strip_prefix('v')
        .or_else(|| raw.trim().strip_prefix('V'))
        .unwrap_or(raw.trim());
    let Some((core, suffix)) = trimmed.split_once('-') else {
        return false;
    };
    !suffix.is_empty() && parse_stable_semver(core).is_some()
}

fn select_windows_setup_url(payload: &serde_json::Value) -> Option<String> {
    let assets = payload["assets"].as_array()?;
    assets.iter().find_map(|asset| {
        let name = asset["name"].as_str()?;
        let url = asset["browser_download_url"].as_str()?;
        if name.ends_with("-Setup.exe") && is_trusted_release_url(url) {
            Some(url.to_string())
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn sample_release(tag: &str, draft: bool, prerelease: bool) -> serde_json::Value {
        serde_json::json!({
            "tag_name": tag,
            "draft": draft,
            "prerelease": prerelease,
            "html_url": format!("https://github.com/vananh2801/eqntide/releases/tag/{tag}"),
            "assets": [
                {
                    "name": "eqntide-native-preview-0.12.0-x86_64-pc-windows-msvc.zip",
                    "browser_download_url": "https://github.com/vananh2801/eqntide/releases/download/v0.12.0/eqntide-native-preview-0.12.0-x86_64-pc-windows-msvc.zip"
                },
                {
                    "name": "EqnTide-0.12.0-Setup.exe",
                    "browser_download_url": "https://github.com/vananh2801/eqntide/releases/download/v0.12.0/EqnTide-0.12.0-Setup.exe"
                },
                {
                    "name": "eqntide-native-preview_0.12.0_amd64.deb",
                    "browser_download_url": "https://github.com/vananh2801/eqntide/releases/download/v0.12.0/eqntide-native-preview_0.12.0_amd64.deb"
                }
            ]
        })
    }

    #[test]
    fn newer_stable_release_selects_windows_setup_and_linux_release_page() {
        let payload = sample_release("v0.12.0", false, false);
        let win = evaluate_release_payload(&payload, "0.11.0", true).unwrap();
        assert_eq!(
            win,
            UpdateCheckOutcome::Available(UpdateRelease {
                version: "0.12.0".into(),
                release_url: "https://github.com/vananh2801/eqntide/releases/tag/v0.12.0".into(),
                download_url: "https://github.com/vananh2801/eqntide/releases/download/v0.12.0/EqnTide-0.12.0-Setup.exe".into(),
            })
        );

        let linux = evaluate_release_payload(&payload, "0.11.0", false).unwrap();
        assert_eq!(
            linux,
            UpdateCheckOutcome::Available(UpdateRelease {
                version: "0.12.0".into(),
                release_url: "https://github.com/vananh2801/eqntide/releases/tag/v0.12.0".into(),
                download_url: "https://github.com/vananh2801/eqntide/releases/tag/v0.12.0".into(),
            })
        );
    }

    #[test]
    fn equal_and_older_releases_report_up_to_date() {
        let equal = sample_release("v0.12.0", false, false);
        assert_eq!(
            evaluate_release_payload(&equal, "0.12.0", true).unwrap(),
            UpdateCheckOutcome::UpToDate
        );

        let older = sample_release("v0.10.9", false, false);
        assert_eq!(
            evaluate_release_payload(&older, "0.11.0", false).unwrap(),
            UpdateCheckOutcome::UpToDate
        );

        let newer_major = sample_release("v1.0.0", false, false);
        assert!(matches!(
            evaluate_release_payload(&newer_major, "0.11.0", false).unwrap(),
            UpdateCheckOutcome::Available(_)
        ));
    }

    #[test]
    fn running_prerelease_sees_matching_stable_as_available_and_older_stable_as_up_to_date() {
        let matching_stable = sample_release("v0.12.0", false, false);
        assert_eq!(
            evaluate_release_payload(&matching_stable, "0.12.0-beta.1", true).unwrap(),
            UpdateCheckOutcome::Available(UpdateRelease {
                version: "0.12.0".into(),
                release_url: "https://github.com/vananh2801/eqntide/releases/tag/v0.12.0".into(),
                download_url: "https://github.com/vananh2801/eqntide/releases/download/v0.12.0/EqnTide-0.12.0-Setup.exe".into(),
            })
        );

        let older_stable = sample_release("v0.11.0", false, false);
        assert_eq!(
            evaluate_release_payload(&older_stable, "0.12.0-beta.1", false).unwrap(),
            UpdateCheckOutcome::UpToDate
        );

        let remote_prerelease = sample_release("v0.12.0-rc.1", false, false);
        assert_eq!(
            evaluate_release_payload(&remote_prerelease, "0.12.0-beta.1", true).unwrap(),
            UpdateCheckOutcome::UpToDate
        );
    }

    #[test]
    fn draft_prerelease_and_malformed_tags_are_never_offered() {
        let draft = sample_release("v0.12.0", true, false);
        assert_eq!(
            evaluate_release_payload(&draft, "0.11.0", true).unwrap(),
            UpdateCheckOutcome::UpToDate
        );

        let prerelease = sample_release("v0.12.0", false, true);
        assert_eq!(
            evaluate_release_payload(&prerelease, "0.11.0", true).unwrap(),
            UpdateCheckOutcome::UpToDate
        );

        let rc_tag = sample_release("v0.12.0-rc.1", false, false);
        assert_eq!(
            evaluate_release_payload(&rc_tag, "0.11.0", true).unwrap(),
            UpdateCheckOutcome::UpToDate
        );

        let bad_tag = sample_release("release-next", false, false);
        assert!(evaluate_release_payload(&bad_tag, "0.11.0", true).is_err());
    }

    #[test]
    fn untrusted_urls_are_rejected_or_ignored() {
        let bad_asset = serde_json::json!({
            "tag_name": "v0.12.0",
            "draft": false,
            "prerelease": false,
            "html_url": "https://github.com/vananh2801/eqntide/releases/tag/v0.12.0",
            "assets": [{
                "name": "EqnTide-0.12.0-Setup.exe",
                "browser_download_url": "https://evil.example.com/EqnTide-0.12.0-Setup.exe"
            }]
        });
        let outcome = evaluate_release_payload(&bad_asset, "0.11.0", true).unwrap();
        assert_eq!(
            outcome,
            UpdateCheckOutcome::Available(UpdateRelease {
                version: "0.12.0".into(),
                release_url: "https://github.com/vananh2801/eqntide/releases/tag/v0.12.0".into(),
                download_url: "https://github.com/vananh2801/eqntide/releases/tag/v0.12.0".into(),
            })
        );

        let bad_html = serde_json::json!({
            "tag_name": "v0.12.0",
            "draft": false,
            "prerelease": false,
            "html_url": "javascript:alert(1)",
            "assets": []
        });
        assert!(evaluate_release_payload(&bad_html, "0.11.0", false).is_err());
    }

    #[test]
    fn check_latest_release_handles_mock_http_success_and_errors() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body = sample_release("v0.12.0", false, false).to_string();

        let server = std::thread::spawn(move || {
            for (status_line, resp_body) in [
                ("200 OK", body.as_str()),
                ("403 Forbidden", "{}"),
                ("404 Not Found", "{}"),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = [0_u8; 1024];
                let _ = stream.read(&mut buf);
                let reply = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp_body}",
                    resp_body.len()
                );
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });

        let url = format!("http://{addr}/releases/latest");
        let ok = check_latest_release(&url, "0.12.0-beta.1", true).unwrap();
        assert!(matches!(ok, UpdateCheckOutcome::Available(_)));

        let rate_err = check_latest_release(&url, "0.11.0", true).unwrap_err();
        assert!(rate_err.contains("rate limit"));

        let missing_err = check_latest_release(&url, "0.11.0", true).unwrap_err();
        assert!(missing_err.contains("No published stable release"));

        server.join().unwrap();
    }
}
