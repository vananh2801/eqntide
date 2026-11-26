//! Build-time desktop release version resolution and SemVer pre-release validation.

use std::sync::OnceLock;

static APP_VERSION: OnceLock<String> = OnceLock::new();

/// Returns the effective desktop application version, including any matching pre-release tag suffix.
pub(crate) fn app_version() -> &'static str {
    APP_VERSION
        .get_or_init(|| {
            resolve_app_version(env!("CARGO_PKG_VERSION"), env!("EQNTIDE_BUILD_TAG_RAW"))
        })
        .as_str()
}

/// Parses a strict `X.Y.Z` (or `vX.Y.Z`) stable SemVer tuple, rejecting pre-release or build metadata.
pub(crate) fn parse_stable_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let without_v = strip_v_prefix(raw.trim());
    if without_v.contains('-') || without_v.contains('+') {
        return None;
    }
    parse_core_semver(without_v)
}

/// Parses a running application version (`X.Y.Z` or `X.Y.Z-<prerelease>`) into `((major, minor, patch), is_prerelease)`.
pub(crate) fn parse_running_version(raw: &str) -> Option<((u64, u64, u64), bool)> {
    let without_v = strip_v_prefix(raw.trim());
    if without_v.contains('+') {
        return None;
    }
    if let Some((core, suffix)) = without_v.split_once('-') {
        if !is_valid_prerelease_suffix(suffix) {
            return None;
        }
        return parse_core_semver(core).map(|semver| (semver, true));
    }
    parse_core_semver(without_v).map(|semver| (semver, false))
}

fn resolve_app_version(pkg_version: &str, raw_tag: &str) -> String {
    let candidate = strip_v_prefix(raw_tag.trim());
    if let Some((core, suffix)) = candidate.split_once('-') {
        if core == pkg_version && is_valid_prerelease_suffix(suffix) {
            return candidate.to_owned();
        }
    }
    pkg_version.to_owned()
}

fn strip_v_prefix(raw: &str) -> &str {
    raw.strip_prefix('v')
        .or_else(|| raw.strip_prefix('V'))
        .unwrap_or(raw)
}

fn is_valid_prerelease_suffix(suffix: &str) -> bool {
    !suffix.is_empty()
        && suffix.split('.').all(|ident| {
            !ident.is_empty()
                && ident
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn parse_core_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let mut parts = raw.split('.');
    let major = parse_ascii_u64(parts.next()?)?;
    let minor = parse_ascii_u64(parts.next()?)?;
    let patch = parse_ascii_u64(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

fn parse_ascii_u64(part: &str) -> Option<u64> {
    if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_app_version_accepts_matching_prerelease_tags() {
        assert_eq!(
            resolve_app_version("0.12.0", "v0.12.0-beta.1"),
            "0.12.0-beta.1"
        );
        assert_eq!(resolve_app_version("0.12.0", "V0.12.0-rc.2"), "0.12.0-rc.2");
        assert_eq!(
            resolve_app_version("0.12.0", " 0.12.0-alpha-1.0 "),
            "0.12.0-alpha-1.0"
        );
        assert_eq!(resolve_app_version("0.12.0", "v0.12.0"), "0.12.0");
    }

    #[test]
    fn resolve_app_version_falls_back_on_mismatch_non_version_or_invalid_suffix() {
        for candidate in [
            "",
            "main",
            "refs/heads/main",
            "v0.11.0-beta.1",
            "v0.12.1",
            "v0.12.0-",
            "v0.12.0-.beta",
            "v0.12.0-beta.",
            "v0.12.0-beta..1",
            "v0.12.0-beta 1",
            "v0.12.0-beta+meta",
        ] {
            assert_eq!(
                resolve_app_version("0.12.0", candidate),
                "0.12.0",
                "candidate '{candidate}' should fall back to crate version"
            );
        }
    }

    #[test]
    fn parse_running_version_distinguishes_stable_and_prerelease() {
        assert_eq!(parse_running_version("0.12.0"), Some(((0, 12, 0), false)));
        assert_eq!(
            parse_running_version("0.12.0-beta.1"),
            Some(((0, 12, 0), true))
        );
        assert_eq!(parse_running_version("0.12.0-"), None);
        assert_eq!(parse_running_version("0.12.0+meta"), None);
    }

    #[test]
    fn compiled_app_version_always_starts_with_crate_version() {
        assert!(app_version().starts_with(env!("CARGO_PKG_VERSION")));
    }
}
