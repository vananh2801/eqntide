# Spec Delta

## ADDED Requirements

### Requirement: System clipboard integration across Linux and desktop backends
Desktop clipboard operations (`Copy`, `Cut`, and `Paste` on the Equation Canvas, `LaTeX source` editor, and text inputs) SHALL exchange text with the active operating-system clipboard—including Linux Wayland and Linux X11 sessions—rather than confining clipboard contents to an in-process fallback buffer.

#### Scenario: Copying from EqnTide populates the system clipboard for external applications
- **WHEN** the user copies or cuts content from the Equation Canvas or `LaTeX source` editor on Linux (Wayland or X11) or another desktop OS
- **THEN** `egui_winit` dispatches `OutputCommand::CopyText` to an active platform clipboard backend (`smithay-clipboard` on Wayland or `arboard` on X11/Windows/macOS) so external applications can paste the copied text

#### Scenario: Pasting from external applications reads the system clipboard into EqnTide
- **WHEN** the user copies text in an external application and presses the paste shortcut inside EqnTide's Equation Canvas or `LaTeX source` editor
- **THEN** `egui_winit` reads the text from the operating-system clipboard and delivers `egui::Event::Paste` with that external text rather than returning stale in-process clipboard data
