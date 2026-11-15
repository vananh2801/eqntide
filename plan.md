# Plan for Full MathType Palette Coverage in EqnTide

> Restored from the Codex session (`Create app layout from sketch`) and updated to reflect the actual state of the repository.

## 1. Goals and Layout

Add all symbols and template variants from the 20 MathType reference sheets, including insertion behavior, canvas editing, LaTeX command suggestions, and MathType/OMML export.

- The sidebar uses a scrollable category list; **Batch DOCX stays pinned at the bottom**, outside the scroll area.
- Keep the toolbar palette at exactly **3 rows** high with the current button size, spacing, and corner radius. Categories with more items scroll horizontally without adding a fourth row.
- Order of the 20 categories:
  1. Fences (`Fences`)
  2. Fractions & Radicals (`Fractions`)
  3. Scripts (`Scripts`)
  4. Integrals (`Integrals`)
  5. Sums (`Sums`)
  6. Products & Big Operators (`BigOperators`)
  7. Matrices (`Matrices`)
  8. Embellishments (`Embellishments`)
  9. Bars (`Bars`)
  10. Labeled Arrows (`LabeledArrows`)
  11. Boxes (`Boxes`)
  12. Relations (`Relations`)
  13. Operators (`Operators`)
  14. Arrows (`Arrows`)
  15. Logic (`Logic`)
  16. Sets (`Sets`)
  17. Symbols (`Symbols`)
  18. Lowercase Greek (`GreekLower`)
  19. Uppercase Greek (`GreekUpper`)
  20. Spaces & Dots (`Spaces`)
- Keep existing capabilities, including templates not shown in the reference sheets. Tooltips use English labels and display the corresponding LaTeX command.

## 2. Shared Catalog and Insertion Behavior

- Create a shared catalog ([`src/catalog.rs`](src/catalog.rs)) for toolbar buttons, thumbnails, and canvas command completion. Each entry has a stable ID, group, English label, command/alias, and LaTeX/MathML template; remove separate duplicated symbol lists in completion.
- **Symbols:** add relations, operators, arrow directions/styles, logic, sets, geometry symbols, styled letters (`\mathbb`, `\mathcal`, `\mathfrak`), full Greek coverage, and variant forms (`\varepsilon`, `\varphi`, `\vartheta`, `\varpi`, `\varrho`, `\varsigma`, `\varkappa`, `\digamma`, `\varTheta`, `\varUpsilon`, `\lambdabar`, etc.).
- **Fences:** add single-sided delimiters, paired delimiters from the reference sheets (including half-open `[ )`, `( ]`, `] [`, `[ [`, `] ]`, bra `⟨ |`, ket `| ⟩`, double brackets `⟦ ⟧`, `⟦`, `⟧`, bra-ket `⟨ | ⟩`), over/under braces/brackets, and label slots.
- **Fractions, roots, scripts:** horizontal/bevelled/stacked-without-bar fractions, normal and small sizes, indexed roots, long division with/without quotient, left/right scripts, and stacked/combined scripts.
- **Integrals and big operators:** single/double/triple/contour forms from the reference sheets; lower, upper, or both limits; side (`\nolimits`) or stacked (`\limits`) placement. Apply the same pattern to sums, products, coproducts, big unions, and big intersections.
- **Embellishments and bars:** add 1–4 over/under dots, hats, arcs, tildes, vectors, hooks, acute/grave accents, single/double overbars and underbars, strike-throughs, and stretchy labeled arrows.
- **Matrices:** add row/column presets (including 4×4), continuation-dot matrices (`\cdots`, `\vdots`, `\ddots`), `Bmatrix`/`Vmatrix`, and a **“Custom matrix”** button (default 2×2, 1–20 rows/columns, left/center/right alignment, and delimiter type; canceling leaves the equation unchanged).
- **Boxes:** closed boxes, rounded boxes, and partial open-side combinations (`top left`, `top right`, `bottom left`, `bottom right`, `top bottom left`, `top bottom right`, `top left right`, `bottom left right`).

Insertion rules: templates with empty slots wrap the current selection into the primary body slot; scripts and decorations attach to the expression before the caret when nothing is selected (creating an empty slot if needed); each insertion is a single undo step and returns focus to the canvas.

## 3. LaTeX, Rendering, and Office Export

- Use the MathML tree as the single source of truth; thumbnails and the canvas share the same renderer.
- Extend the shared parser ([`src/latex_extensions.rs`](src/latex_extensions.rs)) and serializer ([`src/mathml_to_latex.rs`](src/mathml_to_latex.rs)) to preserve semantic attributes: fraction styles (`\dfrac`, `\tfrac`, `\sfrac`, `\tsfrac`, `\atopfrac`), left scripts (`\prescript`), font styles (`\mathbb`, `\mathfrak`, `\mathcal`, `\mathbf`, `\mathrm`, `\boldsymbol`), limit placement (`\limits`/`\nolimits`), accents (`\overaccent`, `\underaccent`, `\doubleoverline`, `\doubleunderline`, …), boxes/strikes (`\boxed`, `\cancel`, `\bcancel`, `\xcancel`, `\enclose{notation}{...}`), column-aligned matrices (`\begin{array}{...}`), `Bmatrix`/`Vmatrix`, and explicit spaces (`\hspace`, `\,`, `\:`, `\;`, `\quad`, `\qquad`).
- Completion reads commands and aliases from the catalog; keep mouse selection and `↑`/`↓`, `Tab`, `Enter`, `Space` interactions.
- Add [`src/render/layout/enclosure.rs`](src/render/layout/enclosure.rs) to the renderer for boxes/strikes, over/under embellishments, bevelled fractions, explicit spaces, and matrix alignment.
- Handle all new templates in the MathType MTEF and OMML encoders/decoders (limits, prescripts, boxes, strikes, labeled arrows, bevelled/small fractions, spaces).
- Report syntax errors in the status bar while keeping the canvas bright and preserving the previous valid equation. Do not change the current bridge protocol or session format.

## 4. Current Status & Completed Items

### Completed in the Repository
- [x] [`src/catalog.rs`](src/catalog.rs): defined 20 groups, `catalog!` macro, `SYMBOLS`, `ENTRIES`, `PALETTE`, `COMMANDS`, `template()`, `preview_template()`, `attaches_to_previous()`, `matrix()`.
- [x] [`desktop/src/app.rs`](desktop/src/app.rs), [`desktop/src/palette.rs`](desktop/src/palette.rs), [`desktop/src/commands.rs`](desktop/src/commands.rs): independently scrolling 20-group sidebar with pinned Batch DOCX button, 3-row horizontally scrollable palette, Custom matrix dialog, catalog-backed completion.
- [x] [`src/latex_extensions.rs`](src/latex_extensions.rs) & [`src/mathml_to_latex.rs`](src/mathml_to_latex.rs): extended parser/serializer and audited character/LaTeX command mappings.
- [x] [`src/render/layout/enclosure.rs`](src/render/layout/enclosure.rs) and updates to `layout/*`, `mtef/*`, `omml/*`, `symbols/*`.
- [x] **Compiler, layout & converter fixes:**
  - [`desktop/src/app.rs`](desktop/src/app.rs): updated `batch_stays_visible_while_sidebar_groups_scroll` for the `egui` API and removed `Id::new` deprecation warnings.
  - [`src/latex_extensions.rs`](src/latex_extensions.rs): used alphabetic `EQNTIDENATIVE...` markers so `latex2mathml::parse_text` reads `Token::Letter`; added `\begin{Bmatrix}` / `\begin{Vmatrix}` parsing.
  - [`src/mathml_to_latex.rs`](src/mathml_to_latex.rs): avoided extra trailing spaces after `\pi` before `}`; mapped `\u{2329}`/`\u{232A}` to `\langle`/`\rangle`.
  - [`src/render/layout/enclosure.rs`](src/render/layout/enclosure.rs): fixed the `1em` at `12pt` test assertion with `PX_PER_PT = 8.0`.
  - [`src/render/layout/fence.rs`](src/render/layout/fence.rs): supported stretching single-sided fences (`\left\llbracket ... \right.`, `\left( ... \right.`, `\left| ... \right.`) and preserved `RawHit` in `stretch_pair`.
  - [`src/mtef/encode.rs`](src/mtef/encode.rs) & [`src/mtef/decode.rs`](src/mtef/decode.rs): recognized stretchy fence `<mrow>` output from `latex2mathml` for `TM_DIRAC`/`TM_INTERVAL`/`TM_OBRACK`, fixed `tmINTERVAL` bitmask decoding (`tvINTV_LEFT_*`/`tvINTV_RIGHT_*`), and added the `0x27C0..=0x27EF` range.
- [x] **Verified all 20 MathType reference sheets against [`src/catalog.rs`](src/catalog.rs):**
  - Full coverage of symbols, half-open brackets `[ )`, `( ]`, `] [`, `[ [`, `] ]`, bra-ket `⟨ |`, `| ⟩`, `⟨ | ⟩`, double brackets `⟦ ⟧`, `⟦`, `⟧`, `4×4` matrices, labeled arrows, open-sided boxes, `\lambdabar`, and font glyph availability checks.
- [x] **Comprehensive acceptance testing:**
  - Round-trip `MathML → LaTeX → MathML`, `MathML ↔ MTEF`, `MathML ↔ OMML`, and `render_interactive` for all catalog entries, plus semantic attribute tests (`\limits`/`\nolimits`, `\dfrac`/`\tfrac`/`\sfrac`/`\tsfrac`/`\atopfrac`, `\prescript`, `mathvariant`, accents/boxes/strikes, matrices, labeled arrows, spaces, fences).
  - Tested selection wrapping insertion, nested insertion, placeholder navigation, undo/redo, completion, invalid LaTeX handling, and multi-DPI UI layouts.
  - Full workspace verification (`cargo fmt --check`, `cargo clippy --workspace --all-targets --locked`, `cargo test --workspace --locked`, `cargo build --workspace --locked`).
