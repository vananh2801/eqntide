use super::super::*;
use super::helpers::*;

#[test]
fn clipboard_feature_and_linux_backends_are_wired() {
    let desktop_manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let eframe_line = desktop_manifest
        .lines()
        .find(|line| line.trim_start().starts_with("eframe ="))
        .expect("eframe dependency line in desktop/Cargo.toml");
    assert!(
        eframe_line.contains("\"clipboard\""),
        "eframe must enable the \"clipboard\" feature in desktop/Cargo.toml"
    );

    let egui_winit_line = desktop_manifest
        .lines()
        .find(|line| line.trim_start().starts_with("egui-winit ="))
        .expect("egui-winit dependency line in desktop/Cargo.toml");
    assert!(
        egui_winit_line.contains("\"clipboard\""),
        "egui-winit must enable the \"clipboard\" feature in desktop/Cargo.toml"
    );

    let lockfile = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.lock"));
    let egui_winit_block = lockfile
        .split("[[package]]")
        .find(|block| block.contains("name = \"egui-winit\""))
        .expect("egui-winit package entry in Cargo.lock");
    assert!(
        egui_winit_block.contains("\"arboard\""),
        "Cargo.lock must resolve arboard under egui-winit for X11/Windows/macOS clipboard"
    );
    assert!(
        egui_winit_block.contains("\"smithay-clipboard\""),
        "Cargo.lock must resolve smithay-clipboard under egui-winit for Linux Wayland clipboard"
    );
}

#[test]
fn canvas_clipboard_copy_cut_and_paste_emit_platform_commands_and_edit_math() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    app.load_math("<math><mi>x</mi><mo>+</mo><mn>1</mn></math>", None, None);

    // Prime two frames so the Equation Canvas widget acquires initial focus.
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert!(ctx.memory(|mem| mem.has_focus(App::editor_focus_id(StartupFocus::Canvas))));

    // Copy selected expression emits OutputCommand::CopyText for the OS clipboard.
    app.editor.select_all();
    let copy_out = run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::Copy]);
    assert!(copy_out
        .platform_output
        .commands
        .iter()
        .any(|cmd| matches!(cmd, egui::OutputCommand::CopyText(text) if text == "x+1")));
    assert_eq!(app.editor.math().to_latex(), "x+1");

    // Cut selected expression emits OutputCommand::CopyText and clears the selection.
    app.editor.select_all();
    let cut_out = run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::Cut]);
    assert!(cut_out
        .platform_output
        .commands
        .iter()
        .any(|cmd| matches!(cmd, egui::OutputCommand::CopyText(text) if text == "x+1")));
    assert_eq!(app.editor.math().to_latex(), "");

    // Paste LaTeX, MathML, and plain text from the OS clipboard into the canvas.
    let _ = run_ui_pass_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Paste(r"\frac{a}{b}".into())],
    );
    assert_eq!(app.editor.math().to_latex(), r"\frac{a}{b}");

    app.editor.select_all();
    let _ = run_ui_pass_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Paste("<math><mi>y</mi></math>".into())],
    );
    assert_eq!(app.editor.math().to_latex(), "y");

    let _ = run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::Paste("z".into())]);
    assert_eq!(app.editor.math().to_latex(), "yz");
}

#[test]
fn latex_source_clipboard_paste_and_copy_round_trip() {
    let ctx = egui::Context::default();
    let mut app = super::helpers::app();
    let latex_id = App::editor_focus_id(StartupFocus::Latex);

    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    ctx.memory_mut(|mem| mem.request_focus(latex_id));
    let _ = run_ui_pass_frame(&ctx, &mut app, vec![]);
    assert!(ctx.memory(|mem| mem.has_focus(latex_id)));

    let _ = run_ui_pass_frame(&ctx, &mut app, vec![egui::Event::Paste("a^2+b^2".into())]);
    assert_eq!(app.latex, "a^2+b^2");

    let select_all = egui::Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    };
    let copy_out = run_ui_pass_frame(&ctx, &mut app, vec![select_all, egui::Event::Copy]);
    assert!(copy_out
        .platform_output
        .commands
        .iter()
        .any(|cmd| matches!(cmd, egui::OutputCommand::CopyText(text) if text == "a^2+b^2")));
}
