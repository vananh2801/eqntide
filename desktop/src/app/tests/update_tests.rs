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
    let cards = card_rects_for_section_titles(
        &about_out,
        tokens,
        &[concat!("EqnTide ", env!("CARGO_PKG_VERSION"))],
    );
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
            "https://github.com/vananh2801/eqntide/releases/download/v0.12.0/EqnTide-0.12.0-Setup.exe"
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
    assert!(up_to_date_texts
        .iter()
        .any(|text| text.contains("is up to date.")));

    app.update_state = UpdateState::Error("Could not reach GitHub".into());
    let err_out = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let err_texts = all_texts(&err_out);
    assert!(err_texts
        .iter()
        .any(|text| text == "Could not reach GitHub"));
    assert!(err_texts.iter().any(|text| text == "Retry"));
    assert!(err_texts.iter().any(|text| text == "Open Releases"));

    let release = UpdateRelease {
        version: "0.12.0".into(),
        release_url: "https://github.com/vananh2801/eqntide/releases/tag/v0.12.0".into(),
        download_url:
            "https://github.com/vananh2801/eqntide/releases/download/v0.12.0/EqnTide-0.12.0-Setup.exe"
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
    assert!(clicked_out.platform_output.commands.iter().any(|cmd| {
        matches!(cmd, egui::OutputCommand::OpenUrl(open) if open.url == release.download_url)
    }));

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
