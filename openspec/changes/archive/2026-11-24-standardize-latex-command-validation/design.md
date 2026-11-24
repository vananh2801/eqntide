# Design

## Context

See `proposal.md` for motivation and scope. Structurally, three gaps existed across [commands.rs](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/commands.rs), [app.rs](file:///home/vananh2801/Documents/mathtype-convert/eqntide/desktop/src/app.rs), [catalog.rs](file:///home/vananh2801/Documents/mathtype-convert/eqntide/src/catalog.rs), and [mathml_to_latex.rs](file:///home/vananh2801/Documents/mathtype-convert/eqntide/src/mathml_to_latex.rs):

1. `desktop/src/app.rs::commit_command` only guarded `text` (`command.name == "text"` and `input.text_argument()`). Every other command in `catalog::COMMANDS` fell through to `palette::template(command.source)`. Because `catalog::template` substitutes `#?` with `\text{EQNTIDEINSERTIONSLOT}` and `latex_extensions` does not attach `mathvariant` to `mtext`, templates like `\mathbb{#?}`, `\mathfrak{#?}`, and `\mathcal{#?}` silently degraded into an empty `<mrow/>`. Meanwhile `\hspace` was hardcoded to `\hspace{1em}` and `\enclose` to `\enclose{box}{#?}`, and `\textstyle`/`\displaystyle` accepted empty trailing input in `latex_extensions` as `<mstyle><mrow/></mstyle>`.
2. `latex_extensions` applies `mathvariant` recursively to every `mi`/`mo`/`mn` child inside font commands, so `\mathbb{1^2}` silently built `<msup><mn mathvariant="double-struck">1</mn><mn mathvariant="double-struck">2</mn></msup>`, which loses double-struck styling in MathType (`Euclid Math Two` only has `A–Z` and `k`) and renders inconsistently. Meanwhile `mathml_to_latex` previously inspected `mathvariant` only on individual leaf tokens, causing compound expressions like `\mathbf{x^2 + 2x - 1 = 0}` to serialize back into `LaTeX source` as `\mathbf{x}^{\mathbf{2}}\mathbf{+}\mathbf{2}...`, and always serialized `mathvariant="script"` as `\mathcal{...}` even for `\mathscr{a}` (which desktop validation rejects under `\mathcal`).
3. Equation Canvas command errors wrote plain text to `self.status` (without the `LaTeX error: ` prefix or `error_fg_color`, shadowed by target warnings, and with different parser fallback messages), whereas `LaTeX source` errors rendered via `latex_error` in the bottom status bar.

## Goals / Non-Goals

**Goals:**
- Unify all non-structural, argument-requiring LaTeX commands in desktop editing under one deterministic argument-completeness and content-validation pipeline.
- Surface identical `LaTeX error: ...` status-bar feedback (`error_fg_color`) for invalid commands from both the Equation Canvas command popup and the `LaTeX source` box, clearing cleanly on buffer edits or cancelation without duplicating errors inside the floating command popup.
- Serialize compound styled expressions back to a single enclosing LaTeX command (e.g., `\mathbf{x^{2}+2x-1=0}`) and preserve `\mathscr` vs `\mathcal` distinctions on desktop roundtrips.
- Keep `COMMANDS` catalog tests (`every_completion_is_an_insertable_template`) and `PALETTE` tests intact without breaking existing toolbar templates.
- Enforce specialized font-alphabet constraints on desktop authoring without breaking legacy DOCX/OMML/MTEF document conversion.

**Non-Goals:**
- Changing structural templates (`\frac`, `\sqrt`, `\boxed`, `\overset`, `\pmatrix`, accents, etc.) — typing their bare name still inserts visible placeholder boxes on the canvas.
- Modifying core `parse_latex_to_mathml` to reject already-authored external DOCX/Word/MathType equations containing non-standard `\mathbb` / `\mathcal` / `\mathfrak` characters or empty spans during batch conversion.
- Adding new MathType Euclid fonts or synthesizing double-struck digits in MTEF.

## Decisions

### 1. Declarative command validation metadata on desktop (`commands.rs`) rather than ad-hoc special cases in `app.rs`
Instead of repeating `text_argument()` / `has_complete_text_argument()` per command in `app.rs`, introduce a general `is_argument_requiring_command` + `validate_desktop_latex` classification in `desktop/src/commands.rs`:

- `text`: 1 braced arg, space-only `\text{ }` allowed, empty `""` rejected.
- `mathbb`: 1 braced arg, non-empty after trim, ASCII chars only in `A..=Z | 'k'`, no `^`, `_`, `\`, `{`, `}`, operators.
- `mathcal`: 1 braced arg, non-empty after trim, ASCII chars only in `A..=Z | 'l'`.
- `mathscr`: 1 braced arg, non-empty after trim, ASCII chars only in `A..=Z | 'a'..='z'`.
- `mathfrak`: 1 braced arg, non-empty after trim, ASCII chars only in `A..=Z | 'a'..='z'`.
- `mathbf` | `mathrm` | `mathit` | `boldsymbol`: 1 braced arg, non-empty after trim (compound expressions like `\mathbf{x^2}` and `\mathrm{H_2O}` allowed).
- `operatorname`: 1 braced arg, non-empty after trim.
- `hspace`: 1 braced arg, non-empty after trim (parser also validates `em`/`ex`/`mu`/`pt` numeric syntax).
- `textstyle` | `displaystyle`: 1 braced arg (or `{\textstyle ...}` top-level form), non-empty after trim.
- `enclose`: 2 braced args (`{notation}{body}`), both non-empty after trim.

*Alternative considered*: Removing `\mathbb`, `\mathfrak`, `\mathcal` from `catalog::COMMANDS`. Rejected because users expect `\math` -> `Tab` / `↑↓` autocomplete to suggest `\mathbb`, `\mathfrak`, `\mathcal`, `\mathrm`, `\mathbf`, `\mathit`, `\mathscr`.

### 2. Keep desktop validation at the desktop boundary (popup + `LaTeX source`)
Validate desktop inputs in `App::parse_desktop_latex` (calling `commands::validate_desktop_latex` before `parse_latex_to_mathml`), shared by both `commit_command` in `desktop/src/app.rs` and `LaTeX source` background parsing (`source_changed` / `Event::Latex`), rather than rejecting `\mathbb{1^2}` inside `eqntide::service::parse_latex_to_mathml`.

*Rationale*: `parse_latex_to_mathml` is shared with the bridge server, CLI, and batch `.docx` conversions. Rejecting `\mathbb{1}` there could fail conversion of third-party documents. Desktop validation protects authoring at input time while keeping batch conversion resilient.

### 3. Unified status-bar error presentation (`command_error` + `latex_error`) without popup duplication
Store `command_error: Option<String>` on `App` (cleared on any popup buffer edit, suggestion navigation/`Tab`, commit, `Esc`, `source_changed`, or document load) alongside `self.status`, and render `self.command_error.as_deref().filter(|_| self.command_input.is_some()).or(self.latex_error.as_deref())` through the single `LaTeX error: {msg}` (`ui.visuals().error_fg_color`) branch in `status_bar`.

*Rationale*: Showing errors solely in the bottom status bar keeps the floating command popup compact, ensures `LaTeX source` and Equation Canvas errors share identical formatting (`LaTeX error: ...` in `error_fg_color`), identical parser/validator messages (via `App::parse_desktop_latex`), and precedence over target warnings, and requests a repaint when `Enter` fails so the status bar updates immediately. Notice `command_error` stays separate from `latex_error` so an in-progress popup error does not reuse `latex_error`'s draft-save lock semantics independently of `command_input.is_some()`.

### 4. Catalog `COMMANDS` parity without breaking template tests
Add `mathscr`, `mathrm`, `mathbf`, `mathit`, `boldsymbol`, `operatorname`, `textstyle`, `displaystyle` to `catalog::COMMANDS` in `src/catalog.rs` with representative sample templates (e.g., `\mathrm{#?}`, `\operatorname{sgn}`, `{\textstyle #?}`) so `commands::tests::every_completion_is_an_insertable_template` remains green while `commit_command` intercepts bare/incomplete forms via `is_argument_requiring_command`.

*Alternative considered*: Changing `COMMANDS` tuple schema to `Option<&str>` for `source`. That would touch every consumer of `eqntide::catalog::COMMANDS` unnecessarily; keeping valid sample `source` strings and enforcing argument completeness in `desktop/src/commands.rs` minimizes blast radius.

### 5. Subtree and sibling `mathvariant` grouping in `src/mathml_to_latex.rs`
Compute `NodeStyle::{Neutral, Uniform { variant, triggers }, Mixed}` across token and structural nodes (`mrow`, `mfrac`, `msqrt`, `mroot`, `msup`, `msub`, `msubsup`, `mmultiscripts`, `munder`, `mover`, `munderover`, `mfenced`, `mtable`, `mtr`, `mtd`) and propagate `active_variant: Option<&str>` down `format_node` and `format_child_slice`.

*Rationale*:
- A `triggers` flag ensures lone `normal` operators/digits/named functions (e.g., `<mi mathvariant="normal">lcm</mi>`) and lone `italic` identifiers (`<mi mathvariant="italic">x</mi>`) keep their existing non-wrapped output, while non-trivial runs (`\mathbf{x^{2}+2x-1=0}`, `\mathrm{H_{2}O+m/s^{2}}`, `\mathbb{RZ}`) collapse into a single style wrapper.
- Checking `contains_non_l_lowercase_script` on `variant == "script"` emits `\mathscr{...}` instead of `\mathcal{...}` whenever lowercase ASCII letters other than `'l'` appear, keeping `\mathscr{a}` valid across desktop roundtrips while preserving `\mathcal{AL}` and `\mathcal{l}`.

## Risks / Trade-offs

- **Existing tests expecting `suggestions("math")[0]`**: Adding `\mathbf`, `\mathit`, `\mathrm`, `\mathscr` preserves `mathbb` first in prefix `"math"` (`'b'` before `'c'`, `'f'`, `'i'`, `'m'`), keeping existing `completion_accept_keys_work_through_the_focused_canvas` (`\math` -> `Tab` -> `mathbb`) intact.
- **Standalone `{\textstyle x}` vs `\textstyle{x}` in `LaTeX source`**: Both `{\textstyle x}` and `\textstyle{x}` are valid; desktop `LaTeX source` validator recognizes `\textstyle{...}` / `{\textstyle ...}` without rejecting braced declarations like `{\textstyle \frac{1}{2}}` while still rejecting bare `\textstyle` or empty `\textstyle{}` / `{\textstyle }`.
- **Nested braces / escaped characters in command arguments**: Walking brace depth in `desktop/src/commands.rs` respects `\` escapes (e.g., `\text{\{a\}}`) identically to `latex_extensions::Scanner::argument`, and validates balanced trailing content when multiple arguments or trailing expressions appear in the popup buffer (e.g. `\mathbb{R}^2`).
