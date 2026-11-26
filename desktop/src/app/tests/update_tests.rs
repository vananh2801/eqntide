use super::super::*;
use super::helpers::*;
use crate::update::{UpdateCheckOutcome, UpdateRelease, UpdateState};

fn all_texts(output: &egui::FullOutput) -> Vec<String> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text_shape) => Some(text_shape.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

fn has_open_url(output: &egui::FullOutput, expected_url: &str) -> bool {
    output
        .platform_output
        .commands
        .iter()
        .any(|cmd| matches!(cmd, egui::OutputCommand::OpenUrl(open) if open.url == expected_url))
}

#[test]
fn about_tab_starts_idle_without_starting_update_check() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    let tokens = ThemeTokens::for_mode(true);
    let mut app = super::helpers::app();

    for tab in [0, 1, 2, 3] {
        app.tab = tab;
        let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    }
    let about_out = run_ui_pass_frame(&ctx, &mut app, vec![]);

    assert_eq!(app.update_state, UpdateState::Idle);
    assert_eq!(app.update_check_seq, 0);
    let expected_title = format!("EqnTide {}", crate::version::app_version());
    let cards = card_rects_for_section_titles(&about_out, tokens, &[expected_title.as_str()]);
    let button_pos = find_text_center(&about_out, |text| text == "Check for updates");
    assert!(cards[0].contains(button_pos));
}

#[test]
fn update_check_events_ignore_stale_sequences_and_deduplicate_in_flight() {
    let mut app = super::helpers::app();
    let ctx = egui::Context::default();

    app.update_check_seq = 2;
    app.update_state = UpdateState::Checking;
    app.start_update_check();
    assert_eq!(app.update_check_seq, 2);
    assert_eq!(app.update_state, UpdateState::Checking);

    app.tx
        .send(Event::UpdateCheck(1, Ok(UpdateCheckOutcome::UpToDate)))
        .unwrap();
    app.process_events(&ctx);
    assert_eq!(app.update_state, UpdateState::Checking);

    let release = UpdateRelease {
        version: "0.12.0".into(),
        release_url: "https://github.com/vananh2801/eqntide/releases/tag/v0.12.0".into(),
        download_url:
            "https://github.com/vananh2801/eqntide/releases/download/v0.12.0/eqntide-0.12.0-windows-x64-setup.exe"
                .into(),
    };
    app.tx
        .send(Event::UpdateCheck(
            2,
            Ok(UpdateCheckOutcome::Available(release.clone())),
        ))
        .unwrap();
    app.process_events(&ctx);
    assert_eq!(app.update_state, UpdateState::Available(release));

    app.update_check_seq = 3;
    app.tx
        .send(Event::UpdateCheck(3, Err("Network error".into())))
        .unwrap();
    app.process_events(&ctx);
    assert_eq!(app.update_state, UpdateState::Error("Network error".into()));

    app.update_check_seq = 4;
    app.tx
        .send(Event::UpdateCheck(4, Ok(UpdateCheckOutcome::UpToDate)))
        .unwrap();
    app.process_events(&ctx);
    assert_eq!(app.update_state, UpdateState::UpToDate);
}

#[test]
fn about_update_states_render_expected_controls_and_open_validated_url() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    ctx.set_theme(egui::Theme::Light);
    let mut app = super::helpers::app();
    app.tab = 3;

    app.update_state = UpdateState::Checking;
    let checking_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let checking_texts = all_texts(&checking_out);
    assert!(checking_texts
        .iter()
        .any(|text| text == "Checking for updates…"));

    app.update_state = UpdateState::UpToDate;
    let up_to_date_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let up_to_date_texts = all_texts(&up_to_date_out);
    assert!(up_to_date_texts.iter().any(|text| text == "Check again"));
    let expected_status = format!("EqnTide {} is up to date.", crate::version::app_version());
    assert!(up_to_date_texts.contains(&expected_status));

    app.update_state = UpdateState::Error("Could not reach GitHub".into());
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let err_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let err_texts = all_texts(&err_out);
    assert!(err_texts
        .iter()
        .any(|text| text == "Could not reach GitHub"));
    assert!(err_texts.iter().any(|text| text == "Retry"));
    assert!(err_texts.iter().any(|text| text == "Open Releases"));
    let open_releases_pos = find_text_center(&err_out, |text| text == "Open Releases");
    let open_releases_click = run_ui_pass_frame(&ctx, &mut app, click_events(open_releases_pos));
    assert!(has_open_url(
        &open_releases_click,
        crate::update::GITHUB_RELEASES_PAGE_URL
    ));

    let release = UpdateRelease {
        version: "0.12.0".into(),
        release_url: "https://github.com/vananh2801/eqntide/releases/tag/v0.12.0".into(),
        download_url:
            "https://github.com/vananh2801/eqntide/releases/download/v0.12.0/eqntide-0.12.0-windows-x64-setup.exe"
                .into(),
    };
    app.update_state = UpdateState::Available(release.clone());
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let avail_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let avail_texts = all_texts(&avail_out);
    assert!(avail_texts
        .iter()
        .any(|text| text == "Update available: v0.12.0"));
    assert!(avail_texts.iter().any(|text| text == "Release notes"));

    let action_label = if cfg!(target_os = "windows") {
        "Download v0.12.0"
    } else {
        "Open release v0.12.0"
    };
    let action_pos = find_text_center(&avail_out, |text| text == action_label);
    let clicked_out = run_ui_pass_frame(&ctx, &mut app, click_events(action_pos));
    assert!(has_open_url(&clicked_out, &release.download_url));

    let notes_pos = find_text_center(&avail_out, |text| text == "Release notes");
    let notes_click = run_ui_pass_frame(&ctx, &mut app, click_events(notes_pos));
    assert!(has_open_url(&notes_click, &release.release_url));

    // Defense-in-depth: untrusted URLs in state never trigger OpenUrl.
    app.update_state = UpdateState::Available(UpdateRelease {
        version: "0.12.0".into(),
        release_url: "javascript:alert(1)".into(),
        download_url: "https://evil.example.com/setup.exe".into(),
    });
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let bad_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let bad_pos = find_text_center(&bad_out, |text| text == action_label);
    let bad_click = run_ui_pass_frame(&ctx, &mut app, click_events(bad_pos));
    assert!(!bad_click
        .platform_output
        .commands
        .iter()
        .any(|cmd| matches!(cmd, egui::OutputCommand::OpenUrl(_))));
}

#[test]
fn about_repository_hyperlink_and_browser_links_feature_are_wired() {
    let ctx = egui::Context::default();
    ThemeTokens::apply_style(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    let mut app = super::helpers::app();
    app.tab = 3;

    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let about_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let repo_url = env!("CARGO_PKG_REPOSITORY");
    let repo_pos = find_text_center(&about_out, |text| text == repo_url);
    let clicked_out = run_ui_pass_frame(&ctx, &mut app, click_events(repo_pos));
    assert!(has_open_url(&clicked_out, repo_url));

    // Guard against disabling egui-winit's `links` feature when `default-features = false`.
    let desktop_manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let eframe_line = desktop_manifest
        .lines()
        .find(|line| line.trim_start().starts_with("eframe ="))
        .expect("eframe dependency line in desktop/Cargo.toml");
    assert!(
        eframe_line.contains("\"links\""),
        "eframe must enable the \"links\" feature in desktop/Cargo.toml"
    );
    let egui_winit_line = desktop_manifest
        .lines()
        .find(|line| line.trim_start().starts_with("egui-winit ="))
        .expect("egui-winit dependency line in desktop/Cargo.toml");
    assert!(
        egui_winit_line.contains("\"links\""),
        "egui-winit must enable the \"links\" feature in desktop/Cargo.toml"
    );

    let lockfile = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.lock"));
    let egui_winit_block = lockfile
        .split("[[package]]")
        .find(|block| block.contains("name = \"egui-winit\""))
        .expect("egui-winit package entry in Cargo.lock");
    assert!(
        egui_winit_block.contains("\"webbrowser\""),
        "Cargo.lock must resolve webbrowser under egui-winit so OpenUrl opens the browser"
    );
}
