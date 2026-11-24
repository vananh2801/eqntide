# Tasks

## 1. Command Catalog Parity

- [x] 1.1 Add `\mathscr`, `\mathrm`, `\mathbf`, `\mathit`, `\boldsymbol`, `\operatorname`, `\textstyle`, and `\displaystyle` to `catalog::COMMANDS` in `src/catalog.rs` with valid insertable sample templates, and verify `cargo test -p eqntide catalog` and `cargo test -p eqntide-desktop commands::tests::every_completion_is_an_insertable_template` pass.
- [x] 1.2 Audit `catalog::COMMANDS` tests in `src/catalog.rs` and `desktop/src/commands.rs` to ensure every newly added command appears in `suggestions(...)` and `exact(...)`.

## 2. Generalized Command & Font-Alphabet Validation

- [x] 2.1 Implement unified desktop LaTeX command validation in `desktop/src/commands.rs` covering all argument-requiring non-structural commands (`\text`, `\mathbb`, `\mathfrak`, `\mathcal`, `\mathscr`, `\mathrm`, `\mathbf`, `\mathit`, `\boldsymbol`, `\operatorname`, `\hspace`, `\textstyle`, `\displaystyle`, `\enclose`), rejecting bare commands, incomplete braces, and empty `{}` arguments (while allowing whitespace-only `\text{ }`), and verify with unit tests in `desktop/src/commands.rs`.
- [x] 2.2 Implement specialized font-alphabet validation (`\mathbb` allowing `A–Z`/`k`; `\mathcal` allowing `A–Z`/`l`; `\mathscr` and `\mathfrak` allowing `A–Z`/`a–z`; blocking digits, operators, and nested scripts such as `\mathbb{1^2}` or `\mathcal{1}`) while preserving compound expressions inside `\mathbf`, `\mathrm`, `\mathit`, `\boldsymbol` (such as `\mathbf{x^2}` and `\mathrm{H_2O}`), and verify with unit tests in `desktop/src/commands.rs`.

## 3. Desktop Popup, LaTeX Source & Roundtrip Serialization

- [x] 3.1 Update `commit_command`, `command_popup`, and `status_bar` in `desktop/src/app.rs` to enforce the generalized validation rules through `App::parse_desktop_latex`, auto-complete matched bare prefixes to the command name on `Enter` without committing, retain the buffer on failure, and display unified `LaTeX error: ...` status-bar feedback (`command_error` + `error_fg_color`, clearing on edit/cancel/commit without duplicating errors or `Backspace to edit` hints inside the command popup); verify with desktop UI and keyboard tests in `desktop/src/app.rs`.
- [x] 3.2 Apply the same `App::parse_desktop_latex` validation to `LaTeX source` updates (`source_changed` / `Event::Latex` in `desktop/src/app.rs`) so invalid constructs such as `\mathbb{1^2}`, `\text{}`, or bare `\textstyle` report the identical `LaTeX error: ...` message and color on the status bar and prevent saving, while keeping `\text{ }` and `{\textstyle ...}` working; verify with `LaTeX source` and cross-surface status bar parity tests in `desktop/src/app.rs`.
- [x] 3.3 Group consecutive sibling nodes and uniform subtrees sharing `mathvariant` in `src/mathml_to_latex.rs` (including `\mathscr` vs `\mathcal` selection for lowercase script letters other than `'l'`) so style commands typed on the canvas serialize once (`\mathbf{x^{2}+2x-1=0}`) and roundtrip cleanly; verify with unit tests in `src/mathml_to_latex.rs` and canvas roundtrip tests in `desktop/src/app.rs`.

## 4. End-to-End Verification

- [x] 4.1 Run the full workspace test suite (`cargo test --workspace`) and `openspec validate standardize-latex-command-validation` to confirm zero regressions across core conversion, MTEF/OMML roundtrips, and desktop editor tests.
