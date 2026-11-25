//! Authentication token management, constant-time verification, and Host/Origin checks.

use std::path::{Path, PathBuf};

/// Return the per-user local data directory for EqnTide state and tokens.
pub fn user_data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "eqntide", "EqnTide")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
}

/// Return the path to the per-user bridge authentication token file.
pub fn bridge_token_path() -> Option<PathBuf> {
    user_data_dir().map(|dir| dir.join("bridge.token"))
}

/// Load or create the persistent per-user bridge authentication token and sync it
/// to any installed per-user WPS JS Add-in directory.
pub fn load_or_create_bridge_token() -> crate::error::Result<String> {
    let dir = user_data_dir().ok_or_else(|| {
        crate::error::Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not resolve per-user EqnTide data directory",
        ))
    })?;
    let token = load_or_create_bridge_token_in(&dir)?;
    sync_wps_addin_token(&token);
    Ok(token)
}

/// Load or create the persistent per-user bridge token inside `dir`.
pub fn load_or_create_bridge_token_in(dir: &Path) -> crate::error::Result<String> {
    ensure_private_dir(dir)?;
    let path = dir.join("bridge.token");
    if let Ok(raw) = std::fs::read_to_string(&path) {
        let trimmed = raw.trim();
        if is_valid_token_format(trimmed) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            }
            return Ok(trimmed.to_owned());
        }
    }
    let token = generate_random_token();
    write_private_file(&path, format!("{token}\n").as_bytes())?;
    Ok(token)
}

/// Validate that the `Host` header resolves strictly to local loopback (anti-DNS-rebinding).
pub fn is_valid_host(host: &str) -> bool {
    let host_trimmed = host.trim();
    let host_part = if host_trimmed.starts_with('[') {
        if let Some(end) = host_trimmed.find(']') {
            &host_trimmed[1..end]
        } else {
            host_trimmed
        }
    } else {
        host_trimmed.split(':').next().unwrap_or(host_trimmed)
    };

    matches!(host_part, "127.0.0.1" | "localhost" | "::1")
}

/// Validate that the `Origin` header matches localhost, HTTPS Office Add-in, or WPS origins.
pub fn is_allowed_origin(origin: &str) -> bool {
    let origin = origin.trim();
    if origin.is_empty() || origin.eq_ignore_ascii_case("null") {
        return false;
    }
    // Allow custom local schemes used by WPS Add-in WebViews.
    if origin.starts_with("file://") || origin.starts_with("wps-addon:") {
        return true;
    }

    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    let host_and_port = rest.split('/').next().unwrap_or("").trim();
    let host = if host_and_port.starts_with('[') {
        if let Some(end) = host_and_port.find(']') {
            &host_and_port[1..end]
        } else {
            host_and_port
        }
    } else {
        host_and_port.split(':').next().unwrap_or(host_and_port)
    };

    // 1. Localhost origins (HTTP or HTTPS)
    if matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https");
    }

    // Remote Office/WPS origins must use HTTPS.
    if !scheme.eq_ignore_ascii_case("https") {
        return false;
    }

    // 2. Microsoft Office Add-in origins
    if host.ends_with(".office.com")
        || host == "office.com"
        || host.ends_with(".office365.com")
        || host == "office365.com"
        || host.ends_with(".microsoft.com")
        || host == "microsoft.com"
        || host.ends_with(".live.com")
        || host == "live.com"
        || host.ends_with(".officeapps.live.com")
    {
        return true;
    }

    // 3. WPS Office Add-in origins
    if host.ends_with(".wps.cn")
        || host == "wps.cn"
        || host.ends_with(".wps.com")
        || host == "wps.com"
        || host.ends_with(".kdocs.cn")
        || host == "kdocs.cn"
    {
        return true;
    }

    false
}

pub(super) fn generate_random_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub(super) fn is_valid_token_format(token: &str) -> bool {
    token.len() >= 16 && token.bytes().all(|byte| byte.is_ascii_graphic())
}

pub(super) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&left_byte, &right_byte) in left.iter().zip(right.iter()) {
        diff |= left_byte ^ right_byte;
    }
    diff == 0
}

fn ensure_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

fn write_private_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|parent_dir| !parent_dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    ensure_private_dir(parent)?;
    let temp_path = parent.join(format!(".tmp-{}", uuid::Uuid::new_v4().simple()));
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let write_res = (|| -> std::io::Result<()> {
        let mut file = opts.open(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o600));
        }
        std::fs::rename(&temp_path, path)?;
        Ok(())
    })();
    if write_res.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    write_res
}

fn wps_jsaddons_install_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    #[cfg(target_os = "windows")]
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let base = PathBuf::from(appdata).join("Kingsoft/wps/jsaddons");
        out.push(base.join("EqnTide"));
        out.push(base.join("EqnTide_1.0.0"));
    }
    #[cfg(not(target_os = "windows"))]
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        for sub in [
            ".local/share/Kingsoft/wps/jsaddons",
            ".local/share/Kingsoft/wps_intl/jsaddons",
        ] {
            let base = home.join(sub);
            out.push(base.join("EqnTide"));
            out.push(base.join("EqnTide_1.0.0"));
        }
    }
    out
}

fn sync_wps_addin_token(token: &str) {
    let payload = format!(
        "// Generated per-user EqnTide bridge token\nwindow.EQNTIDE_BRIDGE_TOKEN = \"{token}\";\n"
    );
    for dir in wps_jsaddons_install_dirs() {
        if dir.is_dir() {
            let target = dir.join("bridge-token.js");
            let _ = write_private_file(&target, payload.as_bytes());
        }
    }
}
