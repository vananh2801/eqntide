# Proposal

## Why

In the desktop app's Equation Canvas command popup, only `\text` currently enforces a complete braced argument before `Enter` can commit. Other non-structural commands in the catalog—such as `\mathbb`, `\mathfrak`, and `\mathcal`—still allow bare `Enter` submission and silently insert an empty, invisible `<mrow/>` (because `#?` expands via `\text{EQNTIDEINSERTIONSLOT}`, which receives no `mathvariant` and collapses to `<mrow/>`), while `\textstyle`/`\displaystyle` accept bare submission as an empty `<mstyle/>`, `\hspace` silently hardcodes `1em`, `\enclose` silently defaults to `box`, empty `{}` arguments (such as `\text{}` or `\mathbb{}`) insert invisible empty nodes, and invalid font-alphabet expressions such as `\mathbb{1^2}` or `\mathcal{1}` are accepted without blocking or explanation. Additionally, MathML style attributes attached to individual leaf tokens cause compound style commands such as `\mathbf{x^2 + 2x - 1 = 0}` to serialize back into `LaTeX source` as repeated per-token commands (`\mathbf{x}^{\mathbf{2}}\mathbf{+}...`), and `\mathscr` lowercase letters roundtrip back as `\mathcal`.

## What Changes

- Standardize desktop LaTeX command validation for all argument-requiring non-structural commands: `\text`, `\mathbb`, `\mathfrak`, `\mathcal`, `\mathscr`, `\mathrm`, `\mathbf`, `\mathit`, `\boldsymbol`, `\operatorname`, `\hspace`, `\textstyle`, `\displaystyle`, and `\enclose`.
- Block `Enter` in the Equation Canvas command popup and surface a unified red `LaTeX error: ...` message in the bottom status bar (without duplicating the error inside the floating command popup) whenever any of those commands lacks a complete braced argument, has an empty braced argument (`{}`—preserving whitespace-only `\text{ }` as valid), or supplies an invalid/unsupported argument, clearing the error as soon as the buffer is edited or canceled.
- Validate specialized mathematical alphabet commands (`\mathbb`, `\mathcal`, `\mathscr`, `\mathfrak`) on desktop entry (both the command popup and the `LaTeX source` box) using the same validation and parser error messages so unsupported characters or nested structures like `\mathbb{1^2}`, `\mathcal{1}`, or `\mathfrak{1+2}` are rejected with clear guidance rather than silently losing styling on export or corrupting LaTeX roundtrips, while general style modifiers (`\mathbf`, `\mathrm`, `\mathit`, `\boldsymbol`) continue to accept compound expressions such as `\mathbf{x^2}` and `\mathrm{H_2O}` as long as their braced argument is non-empty.
- Group consecutive sibling nodes and uniform subtrees sharing the same `mathvariant` in `mathml_to_latex` so expressions like `\mathbf{x^2 + 2x - 1 = 0}` serialize as a single enclosing command (`\mathbf{x^{2}+2x-1=0}`), and emit `\mathscr{...}` when a `script` run uses lowercase letters other than `l`.
- Expand the desktop command catalog (`catalog::COMMANDS`) to surface `\mathscr`, `\mathrm`, `\mathbf`, `\mathit`, `\boldsymbol`, `\operatorname`, `\textstyle`, and `\displaystyle` alongside existing commands, and align `hspace` and `enclose` catalog behavior with explicit argument validation.

## Capabilities

### New Capabilities
- (none)

### Modified Capabilities
- `desktop-latex-typing`: Require complete, non-empty, and argument-valid braced forms for all argument-requiring LaTeX commands in the Equation Canvas command popup, surface unified `LaTeX error: ...` status-bar feedback across both Equation Canvas and `LaTeX source` (without duplicating errors inside the command popup), block invalid font-alphabet inputs such as `\mathbb{1^2}` across desktop editing surfaces, and preserve grouped style commands on `LaTeX source` roundtrip.
- `latex-command-standardization`: Extend desktop command catalog coverage and confirmation rules beyond `\text` to all font, style, operator-name, spacing, and enclosure commands requiring explicit braced arguments, and serialize uniform styled expressions as single enclosing style commands.

## Impact

- `src/catalog.rs`: `COMMANDS` inventory (adding missing style/operator/display commands and ensuring command metadata supports argument-required validation without breaking palette/command template tests).
- `src/mathml_to_latex.rs`: Subtree and sibling `mathvariant` grouping plus `\mathscr` vs `\mathcal` serialization roundtrip selection.
- `desktop/src/commands.rs`: Generalized command-argument parsing and validation helpers replacing the `\text`-only helper (`text_argument` / `has_complete_text_argument`).
- `desktop/src/app.rs`: `commit_command`, `command_error` / `status_bar` unified `LaTeX error: ...` presentation, and shared `parse_desktop_latex` validation across both the command popup and `LaTeX source`.
- Existing batch/document conversion APIs (`parse_latex_to_mathml` for external DOCX/OMML/MTEF files) remain permissive for already-authored documents so historical documents do not fail to convert.
