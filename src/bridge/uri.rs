//! Custom URI scheme parser and generator (`eqntide://open?...`).
//!
//! Enables Word/WPS Add-ins to launch or focus EqnTide directly
//! via system URL handlers (`window.location.href = "eqntide://open?..."`)
//! when the local bridge server is not yet running.

use std::fmt::Write;

/// Represents an action parsed from an `eqntide://` URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomUriAction {
    pub command: String,
    pub tab: Option<String>,
    pub session_id: Option<String>,
    pub data: Option<String>,
    pub format: Option<String>,
    pub target_mode: Option<String>,
}

/// Parse an `eqntide://` URI string into a [`CustomUriAction`].
///
/// Supported formats:
/// * `eqntide://open`
/// * `eqntide://open?session_id=123&data=...&format=mathml&mode=omml`
pub fn parse_custom_uri(uri: &str) -> Result<CustomUriAction, &'static str> {
    let remainder = uri
        .strip_prefix("eqntide://")
        .or_else(|| uri.strip_prefix("eqntide:"))
        .ok_or("invalid URI scheme, expected eqntide://")?;

    let (command, query) = match remainder.split_once('?') {
        Some((cmd, q)) => (cmd.trim_matches('/'), Some(q)),
        None => (remainder.trim_matches('/'), None),
    };

    let mut tab = None;
    let mut session_id = None;
    let mut data = None;
    let mut format = None;
    let mut target_mode = None;

    if let Some(query_str) = query {
        for pair in query_str.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (key, value) = match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => (pair, ""),
            };
            let decoded_value = url_decode(value);
            match key {
                "tab" => tab = Some(decoded_value),
                "session_id" | "id" => session_id = Some(decoded_value),
                "data" => data = Some(decoded_value),
                "format" => format = Some(decoded_value),
                "mode" | "target_mode" => target_mode = Some(decoded_value),
                _ => {}
            }
        }
    }

    Ok(CustomUriAction {
        command: command.to_owned(),
        tab,
        session_id,
        data,
        format,
        target_mode,
    })
}

/// Format a [`CustomUriAction`] into a standard `eqntide://` URI string.
pub fn format_custom_uri(action: &CustomUriAction) -> String {
    let mut out = format!("eqntide://{}", action.command);
    let mut has_param = false;

    let mut append_param = |key: &str, val: &str| {
        if has_param {
            out.push('&');
        } else {
            out.push('?');
            has_param = true;
        }
        out.push_str(key);
        out.push('=');
        out.push_str(&url_encode(val));
    };

    if let Some(ref tab) = action.tab {
        append_param("tab", tab);
    }
    if let Some(ref id) = action.session_id {
        append_param("session_id", id);
    }
    if let Some(ref d) = action.data {
        append_param("data", d);
    }
    if let Some(ref f) = action.format {
        append_param("format", f);
    }
    if let Some(ref m) = action.target_mode {
        append_param("mode", m);
    }

    out
}

// ── Percent Encoding / Decoding (RFC 3986, Zero-dependency) ───────────────────

fn url_decode(s: &str) -> String {
    let mut bytes = Vec::with_capacity(s.len());
    let mut chars = s.as_bytes().iter().copied();

    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next().unwrap_or(b'0');
            let h2 = chars.next().unwrap_or(b'0');
            if let Some(byte) = hex_pair_to_byte(h1, h2) {
                bytes.push(byte);
            } else {
                bytes.push(b'%');
                bytes.push(h1);
                bytes.push(h2);
            }
        } else if b == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(b);
        }
    }

    String::from_utf8_lossy(&bytes).into_owned()
}

fn hex_pair_to_byte(h1: u8, h2: u8) -> Option<u8> {
    let from_hex = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    };
    let n1 = from_hex(h1)?;
    let n2 = from_hex(h2)?;
    Some((n1 << 4) | n2)
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3 / 2);
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                let _ = write!(out, "%{:02X}", b);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_uri_parse_and_format() {
        let action = CustomUriAction {
            command: "open".into(),
            tab: Some("batch".into()),
            session_id: Some("sess-12345".into()),
            data: Some("<math><mi>x</mi></math>".into()),
            format: Some("mathml".into()),
            target_mode: Some("omml".into()),
        };

        let uri = format_custom_uri(&action);
        assert!(uri.starts_with("eqntide://open?"));

        let parsed = parse_custom_uri(&uri).expect("valid parse");
        assert_eq!(parsed, action);
        assert!(parse_custom_uri("matheditor://open").is_err());
    }
}
