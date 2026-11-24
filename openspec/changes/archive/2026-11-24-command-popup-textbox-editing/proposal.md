# Proposal

## Why

When typing LaTeX commands on the desktop Equation Canvas via `\`, the command popup renders the buffered expression as a static non-interactive label (`\{name}`) and discards `ArrowLeft`, `ArrowRight`, `Home`, `End`, `Delete`, and mouse clicks inside the input line. Users cannot click into the command string or move the cursor left/right to fix typos or edit middle arguments (such as `\lim_{x \to 0}` or `\frac{a}{b}`) like a normal textbox.

## What Changes

- Replace the static command label at the top of the Equation Canvas `\` suggestion popup with an interactive single-line textbox (preserving the fixed leading `\` prefix) that supports cursor movement (`ArrowLeft`, `ArrowRight`, `Home`, `End`, word navigation), text selection, mid-buffer character insertion/deletion (`Backspace`, `Delete`), clipboard actions (`Copy`/`Cut`/`Paste`), and pointer click/drag cursor placement.
- Keep all popup command-completion interactions working seamlessly inside the command textbox: `Up`/`Down` navigate suggestions when a command prefix is active, `Tab` fills the highlighted command name and moves the cursor to the end without committing, `Enter` commits the expression and returns focus to the Equation Canvas, `Esc` cancels and returns focus to the Equation Canvas, `Backspace` on an empty buffer closes the popup, `Space` stays ignored on a bare command prefix while inserting normally inside arguments, and clicking a suggestion row commits that command.
- Keep active editor focus tracking anchored to `Equation canvas` while the command popup textbox is open, closing the popup cleanly if the user clicks outside onto the canvas or switches to `LaTeX source`.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `desktop-latex-typing`: Add full single-line textbox cursor navigation, selection, mid-buffer editing, and pointer interaction inside the `\` command popup while preserving suggestion navigation, `Tab` completion, `Enter`/`Esc` lifecycle, and Equation Canvas focus restoration.

## Impact

- `desktop/src/commands.rs`: Extend `commands::Input` with cursor/editing support so both the interactive popup textbox and headless command input tests stay synchronized.
- `desktop/src/app.rs`: Render an interactive single-line `egui::TextEdit` (with fixed `\` prefix) in `App::command_popup`, coordinate popup-specific keys (`Up`/`Down`/`Tab`/`Enter`/`Esc`/empty-`Backspace`/prefix-`Space`) with textbox cursor & mouse handling, close `command_input` when `latex-source` takes focus, and add/update tests covering cursor movement, mid-string editing, and pointer click positioning in the popup.
- No changes to core LaTeX/MathML/OMML/MTEF conversion or Office add-ins.
