use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;

use super::super::protocol::*;
use super::*;

#[cfg(target_os = "linux")]
#[test]
fn repeated_desktop_child_exits_are_reaped() {
    let mut children = Vec::new();
    for _ in 0..8 {
        let child = std::process::Command::new("/bin/sh")
            .args(["-c", "exit 0"])
            .spawn()
            .unwrap();
        children.push((child.id(), reap_desktop_child(child)));
    }
    for (pid, reaper) in children {
        reaper.join().unwrap();
        assert!(
            !std::path::Path::new(&format!("/proc/{pid}")).exists(),
            "child {pid} must be reaped, not left as a zombie"
        );
    }
}

struct RawHttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
}

fn send_raw_http(addr: SocketAddr, req: &str) -> RawHttpResponse {
    let mut stream = TcpStream::connect(addr).expect("connect to test server");
    stream.write_all(req.as_bytes()).expect("write request");
    let mut response_bytes = Vec::new();
    stream
        .read_to_end(&mut response_bytes)
        .expect("read response");
    let response_str = String::from_utf8_lossy(&response_bytes);

    let (head, body) = response_str
        .split_once("\r\n\r\n")
        .unwrap_or((&response_str, ""));
    let mut lines = head.lines();
    let status_line = lines.next().unwrap_or("");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|token| token.parse().ok())
        .unwrap_or(0);

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }

    RawHttpResponse {
        status,
        headers,
        body: body.to_string(),
    }
}

#[test]
fn origin_policy_after_legacy_desktop_removal() {
    for origin in [
        "",
        "null",
        "NULL",
        "tauri://app",
        "http://tauri.localhost",
        "vscode-webview://123",
        "http://appsforoffice.microsoft.com",
        "http://wps.cn",
    ] {
        assert!(!is_allowed_origin(origin), "should reject origin: {origin}");
    }
    for origin in [
        "http://127.0.0.1:17890",
        "http://localhost:3000",
        "https://appsforoffice.microsoft.com",
        "https://www.wps.cn",
        "wps-addon://eqntide",
    ] {
        assert!(is_allowed_origin(origin), "should allow origin: {origin}");
    }
}

#[test]
fn test_server_health_and_cors() {
    let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
    let addr = server_handle.addr();

    // 1. Test OPTIONS preflight (PNA & CORS)
    let opt_req = format!(
        "OPTIONS /health HTTP/1.1\r\nHost: {addr}\r\nOrigin: http://localhost:3000\r\nAccess-Control-Request-Private-Network: true\r\nConnection: close\r\n\r\n"
    );
    let resp = send_raw_http(addr, &opt_req);
    assert_eq!(resp.status, 204);
    assert_eq!(
        resp.headers
            .get("access-control-allow-private-network")
            .map(|value| value.as_str()),
        Some("true")
    );
    assert_eq!(
        resp.headers
            .get("access-control-allow-origin")
            .map(|value| value.as_str()),
        Some("http://localhost:3000")
    );
    assert!(resp
        .headers
        .get("access-control-allow-headers")
        .is_some_and(|header| header.contains("X-EqnTide-Token")));

    // 2. Test GET /health without Origin does not emit wildcard Access-Control-Allow-Origin
    let get_req = format!("GET /health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    let resp = send_raw_http(addr, &get_req);
    assert_eq!(resp.status, 200);
    assert!(!resp.headers.contains_key("access-control-allow-origin"));
    let health: HealthResponse = serde_json::from_str(&resp.body).expect("json parse");
    assert_eq!(health.status, "ok");
    assert_eq!(health.server, "eqntide-bridge");
}

#[test]
fn test_security_dns_rebinding_and_missing_host_rejected() {
    let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
    let addr = server_handle.addr();

    // Send a request with spoofed/malicious Host header
    let rebinding_req = format!(
        "GET /health HTTP/1.1\r\nHost: evil-attacker.com:{}\r\nConnection: close\r\n\r\n",
        addr.port()
    );
    let resp = send_raw_http(addr, &rebinding_req);
    assert_eq!(resp.status, 403);
    assert!(resp.body.contains("DNS rebinding"));

    // HTTP/1.0 request without Host header must also be rejected
    let no_host_req = "GET /health HTTP/1.0\r\nConnection: close\r\n\r\n";
    let resp = send_raw_http(addr, no_host_req);
    assert_eq!(resp.status, 403);
    assert!(resp.body.contains("missing Host"));
}

#[test]
fn test_security_untrusted_and_null_origin_rejected() {
    let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
    let addr = server_handle.addr();

    for bad_origin in ["https://malicious-website.xyz", "null", "http://office.com"] {
        let req = format!(
            "GET /health HTTP/1.1\r\nHost: {addr}\r\nOrigin: {bad_origin}\r\nConnection: close\r\n\r\n"
        );
        let resp = send_raw_http(addr, &req);
        assert_eq!(resp.status, 403, "origin={bad_origin}");
        assert!(resp.body.contains("untrusted Origin"));
        assert!(!resp.headers.contains_key("access-control-allow-origin"));
    }
}

#[test]
fn test_security_office_add_in_origin_allowed() {
    let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
    let addr = server_handle.addr();

    // Send request from official Microsoft Office Add-in host
    let req = format!(
        "GET /health HTTP/1.1\r\nHost: {addr}\r\nOrigin: https://appsforoffice.microsoft.com\r\nConnection: close\r\n\r\n"
    );
    let resp = send_raw_http(addr, &req);
    assert_eq!(resp.status, 200);
    assert_eq!(
        resp.headers
            .get("access-control-allow-origin")
            .map(|value| value.as_str()),
        Some("https://appsforoffice.microsoft.com")
    );
}

#[test]
fn test_session_lifecycle_longpoll() {
    let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
    let addr = server_handle.addr();
    let token = server_handle.auth_token().to_owned();

    // 1. Open a new session with OMML input
    let raw_omml = r#"<w:p xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\"><m:oMath><m:r><m:t>x+1</m:t></m:r></m:oMath></w:p>"#;
    let open_payload = format!("{{\"data\": \"{}\", \"format\": \"omml\"}}", raw_omml);
    let open_req = format!(
        "POST /session/open HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        open_payload.len(),
        open_payload
    );
    let resp = send_raw_http(addr, &open_req);
    assert_eq!(resp.status, 200);
    let open_res: SessionOpenResponse = serde_json::from_str(&resp.body).expect("json parse");
    assert_eq!(open_res.status, "opened");
    let session_id = open_res.session_id;

    // 2. In background thread, perform the long-poll wait
    let sid_clone = session_id.clone();
    let token_clone = token.clone();
    let wait_handle = std::thread::spawn(move || {
        let wait_req = format!(
            "GET /session/{sid_clone}/wait?timeout=10 HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token_clone}\r\nConnection: close\r\n\r\n"
        );
        let resp = send_raw_http(addr, &wait_req);
        let wait_res: SessionWaitResponse = serde_json::from_str(&resp.body).expect("json parse");
        wait_res
    });

    // 3. Update the session from the editor side
    std::thread::sleep(Duration::from_millis(50));
    let upd_payload =
        r#"{"mathml": "<math><mi>y</mi><mo>=</mo><mn>2</mn></math>", "target_mode": "omml"}"#;
    let upd_req = format!(
        "POST /session/{session_id}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        upd_payload.len(),
        upd_payload
    );
    let resp = send_raw_http(addr, &upd_req);
    assert_eq!(resp.status, 200);
    let upd_res: SessionActionResponse = serde_json::from_str(&resp.body).expect("json parse");
    assert_eq!(upd_res.status, "ok");

    // 4. Verify the waiting thread woke up with the result
    let wait_result = wait_handle.join().expect("thread join");
    assert_eq!(wait_result.status, "ready");
    assert!(wait_result
        .flat_opc
        .as_ref()
        .unwrap()
        .contains("pkg:package"));
    assert!(wait_result.omml.as_ref().unwrap().contains("m:oMath"));
}

#[test]
fn session_output_honors_the_saved_mode_for_insert_and_edit() {
    let server =
        start_bridge_server_with_window_requests("127.0.0.1:0", Some(Arc::new(|_| {}))).unwrap();
    let addr = server.addr();
    let token = server.auth_token().to_owned();
    for (initial, saved, data) in [
        ("mathtype", "omml", ""),
        ("mathtype", "omml", "<math><mi>x</mi></math>"),
        ("omml", "mathtype", ""),
        ("omml", "mathtype", "<math><mi>x</mi></math>"),
    ] {
        let body =
            serde_json::json!({"data":data,"format":"mathml","target_mode":initial}).to_string();
        let response = send_raw_http(addr, &format!(
            "POST /session/open HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()));
        assert_eq!(response.status, 200);
        let opened: SessionOpenResponse = serde_json::from_str(&response.body).unwrap();
        let sid = opened.session_id;
        let body =
            serde_json::json!({"mathml":"<math><mi>y</mi></math>","target_mode":saved}).to_string();
        let response = send_raw_http(addr, &format!(
            "POST /session/{sid}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()));
        assert_eq!(response.status, 200);
        let response = send_raw_http(addr, &format!(
            "GET /session/{sid}/wait?timeout=1 HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nConnection: close\r\n\r\n"));
        assert_eq!(response.status, 200);
        let result: SessionWaitResponse = serde_json::from_str(&response.body).unwrap();
        assert_eq!(result.status, "ready");
        let package = result.flat_opc.unwrap();
        if saved == "omml" {
            assert!(package.contains("m:oMath"));
            assert!(!package.contains("Equation.DSMT4"));
            assert!(result.emf_path.is_none());
            assert!(result.width_pt.is_none());
            assert!(result.height_pt.is_none());
            assert!(result.depth_pt.is_none());
        } else {
            assert!(package.contains("Equation.DSMT4"));
            let path = result.emf_path.expect("MathType must retain EMF insertion");
            let image = std::fs::read(&path).unwrap();
            let mtef = crate::extract_mtef_from_emf(&image).unwrap();
            assert!(crate::mtef::decode::mtef_to_mathml(&mtef).is_some());
            std::fs::remove_file(path).unwrap();
        }
    }
}

#[test]
fn test_convert_snippet_stateless() {
    let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
    let addr = server_handle.addr();
    let token = server_handle.auth_token().to_owned();

    let snip_payload = r#"{"data": "<math><mi>a</mi><mo>+</mo><mi>b</mi></math>", "from": "mathml", "to": "flat_opc_omml"}"#;
    let snip_req = format!(
        "POST /convert/snippet HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        snip_payload.len(),
        snip_payload
    );
    let resp = send_raw_http(addr, &snip_req);
    assert_eq!(resp.status, 200);

    let snip_res: ConvertSnippetResponse = serde_json::from_str(&resp.body).expect("json parse");
    assert_eq!(snip_res.status, "ok");
    assert!(snip_res.result.contains("pkg:package"));
    assert!(snip_res.result.contains("m:oMath"));

    // Test EMF snippet export
    let emf_payload =
        r#"{"data": "<math><mi>x</mi><mo>+</mo><mn>1</mn></math>", "from": "mathml", "to": "emf"}"#;
    let emf_req = format!(
        "POST /convert/snippet HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        emf_payload.len(),
        emf_payload
    );
    let resp_emf = send_raw_http(addr, &emf_req);
    assert_eq!(resp_emf.status, 200);
    let emf_res: ConvertSnippetResponse = serde_json::from_str(&resp_emf.body).expect("json parse");
    assert_eq!(emf_res.status, "ok");
    assert!(!emf_res.result.is_empty());

    // Test PNG snippet export
    let png_payload =
        r#"{"data": "<math><mi>x</mi><mo>+</mo><mn>1</mn></math>", "from": "mathml", "to": "png"}"#;
    let png_req = format!(
        "POST /convert/snippet HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        png_payload.len(),
        png_payload
    );
    let resp_png = send_raw_http(addr, &png_req);
    assert_eq!(resp_png.status, 200);
    let png_res: ConvertSnippetResponse = serde_json::from_str(&resp_png.body).expect("json parse");
    assert_eq!(png_res.status, "ok");
    assert!(!png_res.result.is_empty());
}

#[test]
fn test_session_open_with_mathml_generates_latex() {
    let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
    let addr = server_handle.addr();
    let token = server_handle.auth_token().to_owned();

    let open_payload = r#"{"data": "<math><mi>x</mi><mo>=</mo><mfrac><mi>a</mi><mi>b</mi></mfrac></math>", "format": "mathml", "target_mode": "mathtype"}"#;
    let open_req = format!(
        "POST /session/open HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        open_payload.len(),
        open_payload
    );
    let resp = send_raw_http(addr, &open_req);
    assert_eq!(resp.status, 200);
    let open_res: SessionOpenResponse = serde_json::from_str(&resp.body).expect("json parse");
    assert_eq!(open_res.status, "opened");
    assert_eq!(open_res.latex, Some("x=\\frac{a}{b}".into()));
}

#[test]
fn test_server_serves_addin_files() {
    let server_handle = start_bridge_server("127.0.0.1:0").expect("start test server");
    let addr = server_handle.addr();
    let token = server_handle.auth_token().to_owned();

    // Removed Word web endpoints stay unavailable even on upgraded installs.
    // Word now uses the VBA template and the session/conversion API only.
    for path in [
        "/manifest.xml",
        "/word-addin/manifest.xml",
        "/word-addin/commands.js",
    ] {
        let unauth = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        assert_eq!(send_raw_http(addr, &unauth).status, 401);

        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nConnection: close\r\n\r\n"
        );
        assert_eq!(send_raw_http(addr, &request).status, 404);
    }

    // 2. WPS Add-in files
    let req_wps =
        format!("GET /wps-addin/ribbon.xml HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    let resp_wps = send_raw_http(addr, &req_wps);
    assert_eq!(resp_wps.status, 200);
    assert!(resp_wps.body.contains("TabEqnTide"));
    assert!(resp_wps.body.contains("btnInsertEquation"));

    let req_wps_js =
        format!("GET /wps-addin/main.js HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    let resp_wps_js = send_raw_http(addr, &req_wps_js);
    assert_eq!(resp_wps_js.status, 200);
    assert!(resp_wps_js.body.contains("InsertEquation"));
    assert!(resp_wps_js.body.contains("WordOpenXML"));
}

#[test]
fn native_callback_receives_batch_tab_and_legacy_callback_remains_compatible() {
    let (tx, rx) = std::sync::mpsc::channel();
    let callback: WindowRequestCallback = Arc::new(move |req| {
        tx.send(req).unwrap();
    });
    let server = start_bridge_server_with_window_requests("127.0.0.1:0", Some(callback)).unwrap();
    let addr = server.addr();
    let token = server.auth_token().to_owned();
    let body = r#"{"tab":"batch"}"#;
    let response = send_raw_http(
        addr,
        &format!(
            "POST /window/show HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
    );
    assert_eq!(response.status, 200);
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(2))
            .unwrap()
            .tab
            .as_deref(),
        Some("batch")
    );
    let (tx, rx) = std::sync::mpsc::channel();
    let callback: WindowCallback = Arc::new(move |id, _, _, _| {
        tx.send(id.to_owned()).unwrap();
    });
    let legacy = start_bridge_server_with_callback("127.0.0.1:0", Some(callback)).unwrap();
    let addr = legacy.addr();
    let token = legacy.auth_token().to_owned();
    let response = send_raw_http(
        addr,
        &format!(
            "POST /window/show HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
        ),
    );
    assert_eq!(response.status, 200);
    assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), "");
}

#[test]
fn session_get_preserves_target_and_reports_completed_state() {
    let callback: WindowCallback = Arc::new(|_, _, _, _| {});
    let server = start_bridge_server_with_callback("127.0.0.1:0", Some(callback)).unwrap();
    let token = server.auth_token().to_owned();
    let id = server.session_manager().create_session_with_target(
        "<math><mi>x</mi></math>".into(),
        None,
        Some(14.0),
        "mathtype",
    );
    let addr = server.addr();
    let get = || {
        send_raw_http(
            addr,
            &format!(
                "GET /session/{id} HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nConnection: close\r\n\r\n"
            ),
        )
    };
    let response: serde_json::Value = serde_json::from_str(&get().body).unwrap();
    assert_eq!(response["target_mode"], "mathtype");
    assert_eq!(response["state"], "waiting");
    server.session_manager().cancel_session(&id).unwrap();
    let response: serde_json::Value = serde_json::from_str(&get().body).unwrap();
    assert_eq!(response["state"], "cancelled");
}

#[test]
fn legacy_vba_convert_route_uses_the_native_snippet_service() {
    let server = start_bridge_server("127.0.0.1:0").unwrap();
    let addr = server.addr();
    let token = server.auth_token().to_owned();
    let body = r#"{"data":"x+1","from":"latex","to":"omml"}"#;
    let response = send_raw_http(
        addr,
        &format!(
            "POST /convert HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
    );
    assert_eq!(response.status, 200);
    let json: serde_json::Value = serde_json::from_str(&response.body).unwrap();
    assert!(json["result"].as_str().unwrap().contains("oMath"));
}

#[test]
fn bridge_tracks_longpoll_until_its_response_is_written() {
    let server = start_bridge_server("127.0.0.1:0").unwrap();
    let addr = server.addr();
    let token = server.auth_token().to_owned();
    let id = server
        .session_manager()
        .create_session("<math/>".into(), None, None);
    let sid = id.clone();
    let request = std::thread::spawn(move || {
        send_raw_http(
            addr,
            &format!(
                "GET /session/{sid}/wait?timeout=5 HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nConnection: close\r\n\r\n"
            ),
        )
    });
    let start = std::time::Instant::now();
    while server.pending_requests() == 0 && start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(server.pending_requests(), 1);
    server.session_manager().cancel_session(&id).unwrap();
    assert!(request.join().unwrap().body.contains("cancelled"));
    while server.pending_requests() != 0 && start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(server.pending_requests(), 0);
}

#[test]
fn session_update_validates_target_compatibility() {
    let server = start_bridge_server("127.0.0.1:0").unwrap();
    let addr = server.addr();
    let token = server.auth_token().to_owned();

    // 1. OMML session rejects longdiv with 422
    let omml_sid = server.session_manager().create_session_with_target(
        "<math/>".into(),
        None,
        Some(12.0),
        "omml",
    );
    let body = serde_json::json!({
        "mathml": "<math><menclose notation=\"longdiv\"><mn>144</mn></menclose></math>",
        "target_mode": "omml",
        "font_pt": 12.0
    })
    .to_string();
    let resp = send_raw_http(
        addr,
        &format!(
            "POST /session/{omml_sid}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
    );
    assert_eq!(resp.status, 422, "body: {}", resp.body);

    // 2. MathType session accepts longdiv with 200
    let mt_sid = server.session_manager().create_session_with_target(
        "<math/>".into(),
        None,
        Some(12.0),
        "mathtype",
    );
    let body_mt = serde_json::json!({
        "mathml": "<math><menclose notation=\"longdiv\"><mn>144</mn></menclose></math>",
        "target_mode": "mathtype",
        "font_pt": 12.0
    })
    .to_string();
    let resp_mt = send_raw_http(
        addr,
        &format!(
            "POST /session/{mt_sid}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_mt}",
            body_mt.len()
        ),
    );
    assert_eq!(resp_mt.status, 200, "body: {}", resp_mt.body);
    assert!(server.session_manager().remove_session(&mt_sid));

    // 3. MathType session rejects verticalstrike with 422
    let mt_sid2 = server.session_manager().create_session_with_target(
        "<math/>".into(),
        None,
        Some(12.0),
        "mathtype",
    );
    let body_vs = serde_json::json!({
        "mathml": "<math><menclose notation=\"verticalstrike\"><mi>x</mi></menclose></math>",
        "target_mode": "mathtype",
        "font_pt": 12.0
    })
    .to_string();
    let resp_vs = send_raw_http(
        addr,
        &format!(
            "POST /session/{mt_sid2}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_vs}",
            body_vs.len()
        ),
    );
    assert_eq!(resp_vs.status, 422, "body: {}", resp_vs.body);
}

#[test]
fn unauthenticated_and_wrong_token_requests_are_rejected_with_401() {
    let server = start_bridge_server("127.0.0.1:0").unwrap();
    let addr = server.addr();

    for (method, path, body) in [
        ("GET", "/session/latest", ""),
        ("POST", "/session/open", "{}"),
        ("POST", "/window/show", "{}"),
        ("POST", "/log", "hi"),
        ("POST", "/convert/snippet", "{}"),
        ("POST", "/convert/document", "{}"),
    ] {
        let unauth = format!(
            "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        assert_eq!(send_raw_http(addr, &unauth).status, 401, "{method} {path}");

        let wrong = format!(
            "{method} {path} HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: wrong-token-value\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        assert_eq!(send_raw_http(addr, &wrong).status, 401, "{method} {path}");
    }
}

#[test]
fn addin_static_serving_blocks_traversal_and_drive_prefixes() {
    assert!(!is_safe_relative_addin_path("C:/Windows/win.ini"));
    assert!(!is_safe_relative_addin_path(r"C:\Users\secret.txt"));
    assert!(!is_safe_relative_addin_path("../Cargo.toml"));
    assert!(!is_safe_relative_addin_path("..%2fCargo.toml"));
    assert!(!is_safe_relative_addin_path("bridge-token.js"));
    assert!(!is_safe_relative_addin_path(".hidden"));
    assert!(is_safe_relative_addin_path("main.js"));
    assert!(is_safe_relative_addin_path("assets/icons/tex-32.png"));

    let server = start_bridge_server("127.0.0.1:0").unwrap();
    let addr = server.addr();
    for bad_path in [
        "/wps-addin/C:/Windows/win.ini",
        "/wps-addin/C:%5CWindows%5Cwin.ini",
        "/wps-addin/%2e%2e/Cargo.toml",
        "/wps-addin/../Cargo.toml",
        "/wps-addin/bridge-token.js",
    ] {
        let req = format!("GET {bad_path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        assert_eq!(send_raw_http(addr, &req).status, 404, "{bad_path}");
    }
}

#[test]
fn open_folder_removed_and_document_output_extension_validated() {
    let server = start_bridge_server("127.0.0.1:0").unwrap();
    let addr = server.addr();
    let token = server.auth_token().to_owned();

    let body = r#"{"path":"/tmp"}"#;
    let open_folder = format!(
        "POST /open/folder HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    assert_eq!(send_raw_http(addr, &open_folder).status, 404);

    let dir = std::env::temp_dir().join(format!(
        "eqntide_doc_test_{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let input_docx = dir.join("sample.docx");
    std::fs::write(&input_docx, b"not-a-zip").unwrap();
    let bad_out = dir.join("evil.bat");
    let doc_body = serde_json::json!({
        "input_path": input_docx.to_string_lossy(),
        "output_path": bad_out.to_string_lossy(),
        "target": "omml"
    })
    .to_string();
    let doc_req = format!(
        "POST /convert/document HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{doc_body}",
        doc_body.len()
    );
    assert_eq!(send_raw_http(addr, &doc_req).status, 400);
    assert!(!bad_out.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn invalid_session_update_writes_no_emf_and_oversized_body_returns_413() {
    let server = start_bridge_server("127.0.0.1:0").unwrap();
    let addr = server.addr();
    let token = server.auth_token().to_owned();

    let bogus_id = "not-a-uuid-and-missing";
    let expected_emf = std::env::temp_dir().join(format!("eqntide_{bogus_id}.emf"));
    let _ = std::fs::remove_file(&expected_emf);
    let upd_body = r#"{"mathml":"<math><mi>x</mi></math>","target_mode":"mathtype"}"#;
    let upd_req = format!(
        "POST /session/{bogus_id}/update HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{upd_body}",
        upd_body.len()
    );
    assert_eq!(send_raw_http(addr, &upd_req).status, 404);
    assert!(!expected_emf.exists());

    let too_big = "a".repeat(MAX_BODY_BYTES + 16);
    let big_req = format!(
        "POST /log HTTP/1.1\r\nHost: {addr}\r\n{BRIDGE_TOKEN_HEADER}: {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{too_big}",
        too_big.len()
    );
    assert_eq!(send_raw_http(addr, &big_req).status, 413);
}

#[test]
fn token_persistence_and_stale_session_cleanup_remove_temp_emfs() {
    let dir = std::env::temp_dir().join(format!(
        "eqntide_tok_test_{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let tok1 = load_or_create_bridge_token_in(&dir).unwrap();
    let tok2 = load_or_create_bridge_token_in(&dir).unwrap();
    assert_eq!(tok1, tok2);
    assert!(tok1.len() >= 32);

    let mgr = SessionManager::new();
    let sid = mgr.create_session_with_target("<math/>".into(), None, Some(12.0), "mathtype");
    let emf_path = dir.join(format!("eqntide_{sid}.emf"));
    std::fs::write(&emf_path, b"emf-data").unwrap();
    mgr.update_session(
        &sid,
        "<math/>".into(),
        "<pkg:package/>".into(),
        "<m:oMath/>".into(),
        Some(emf_path.to_string_lossy().into_owned()),
        Some(10.0),
        Some(10.0),
        Some(2.0),
        Some(12.0),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(20));
    mgr.cleanup_stale(Duration::from_millis(5));
    assert!(mgr.get_session(&sid).is_none());
    assert!(!emf_path.exists());
    let _ = std::fs::remove_dir_all(&dir);
}
