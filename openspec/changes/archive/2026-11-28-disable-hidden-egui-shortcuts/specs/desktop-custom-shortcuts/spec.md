# Spec Delta

## ADDED Requirements

### Requirement: Disable global toolkit UI zoom and quit shortcuts
The desktop application SHALL disable built-in global UI-scaling shortcuts (`Ctrl++`, `Ctrl+=`, `Ctrl+-`, `Ctrl+0`, including `Shift`/`Alt` variants) and built-in global quit shortcuts (`Ctrl+Q`, including `Shift`/`Alt` variants) across all rendering backends so customizable chords never rescale the application UI or close the window unexpectedly.

#### Scenario: Global zoom shortcuts do not rescale the desktop UI
- **WHEN** the user presses `Ctrl+-`, `Ctrl++`, `Ctrl+=`, or `Ctrl+0` (with or without `Shift` or `Alt`) anywhere in the desktop application
- **THEN** the global UI scale factor remains `1.0` and the window chrome, sidebar, palette, and footer geometry stay intact

#### Scenario: Palette shortcut bound to Ctrl+Minus or Ctrl+Equals inserts without UI zoom
- **WHEN** the user binds `Ctrl+-` or `Ctrl+=` to a palette entry and presses it while the **Equation Canvas** has focus
- **THEN** the assigned palette template is inserted on the Equation Canvas and the global UI scale remains unchanged

#### Scenario: Ctrl+Q does not trigger an unintended window close
- **WHEN** the user presses `Ctrl+Q` (or `Ctrl+Shift+Q` / `Ctrl+Alt+Q`) anywhere in the desktop application
- **THEN** no window-close request is emitted by the UI toolkit, and if `Ctrl+Q` is bound to a palette entry and the **Equation Canvas** has focus, that template is inserted normally

### Requirement: Suppress Emacs/Readline deletion chords inside text inputs
While a text-editing control has focus and Settings shortcut recording is inactive, the desktop application SHALL suppress `Ctrl+H`, `Ctrl+K`, `Ctrl+U`, and `Ctrl+W` so they do not delete text, while preserving standard editing keys and keeping `Ctrl+H`, `Ctrl+K`, `Ctrl+U`, and `Ctrl+W` active on the **Equation Canvas** and in the Settings shortcut recorder.

#### Scenario: Ctrl+H, Ctrl+K, Ctrl+U, and Ctrl+W do not mutate text inputs
- **WHEN** focus is inside the `LaTeX source` box, Settings shortcut search box, inline `\command` input, or an active numeric `DragValue` text field and the user presses `Ctrl+H`, `Ctrl+K`, `Ctrl+U`, or `Ctrl+W`
- **THEN** the text content and cursor selection remain unmodified

#### Scenario: Standard text-editing and clipboard chords remain functional
- **WHEN** focus is inside a text-editing control such as `LaTeX source` and the user presses `Ctrl+Arrow*`, `Ctrl+Backspace`, `Ctrl+Delete`, `Ctrl+Home`, `Ctrl+End`, or OS clipboard chords (`Ctrl/Shift+Insert`, `Shift+Delete`)
- **THEN** cursor movement, word deletion, and clipboard actions behave normally

#### Scenario: Ctrl+H, Ctrl+K, Ctrl+U, and Ctrl+W remain available on Equation Canvas and in Settings recorder
- **WHEN** the user records `Ctrl+H`, `Ctrl+K`, `Ctrl+U`, or `Ctrl+W` in Settings or presses an assigned one on the focused **Equation Canvas**
- **THEN** Settings records the chord normally and the Equation Canvas dispatches the assigned palette template (such as `Ctrl+H` for `^{#?}`)

### Requirement: Reject preserved editing, clipboard, and navigation shortcuts in Settings
The desktop Settings shortcut recorder and persisted shortcut parser SHALL disallow binding palette shortcuts to preserved text-editing, clipboard, and navigation chords (`Ctrl+Arrow*`, `Ctrl+Backspace`, `Ctrl+Delete`, `Ctrl+Home`, `Ctrl+End`, `Ctrl+Insert`, `Shift+Insert`, `Shift+Delete`, `Tab`, `Shift+Tab`, `Enter`, `Space`, and `Esc`).

#### Scenario: Preserved editing, clipboard, and navigation chords are rejected during Settings recording
- **WHEN** the Settings shortcut recorder is active for a palette entry and the user presses `Ctrl+ArrowLeft`, `Ctrl+ArrowRight`, `Ctrl+Backspace`, `Ctrl+Delete`, `Ctrl+Home`, `Ctrl+End`, `Ctrl+Insert`, `Shift+Insert`, `Shift+Delete`, `Tab`, `Shift+Tab`, `Enter`, or `Space`
- **THEN** no shortcut is bound to that palette entry and Settings either opens the reserved-system conflict modal (for OS clipboard events) or displays a reserved-key validation hint while keeping existing bindings unchanged

#### Scenario: Persisted shortcut overrides containing reserved editing or navigation chords are ignored
- **WHEN** `settings.json` contains a palette shortcut override value such as `"Ctrl+ArrowLeft"`, `"Ctrl+Backspace"`, `"Ctrl+Home"`, `"Ctrl+Insert"`, `"Shift+Insert"`, `"Shift+Delete"`, or `"Tab"`
- **THEN** `ShortcutChord::parse` and `ShortcutMap::from_overrides` ignore that override without binding it
