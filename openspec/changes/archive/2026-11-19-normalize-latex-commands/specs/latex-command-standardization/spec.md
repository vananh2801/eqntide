# Spec Delta

## Purpose

Defines canonical, standard-aligned LaTeX parsing, normalization across MathML/OMML/MTEF representations, and explicit LaTeX serialization while retaining MathML/OMML-only and legacy extension commands.

## ADDED Requirements

### Requirement: Canonical explicit LaTeX serialization
When serializing a MathML equation to LaTeX (`to_latex`), the serializer SHALL emit standard, explicit LaTeX commands wherever a standard TeX, `amsmath`, or `amssymb` form exists rather than abbreviated aliases or custom non-standard macros.

#### Scenario: Relations, arrows, and set-minus serialize to explicit standard commands with clean spacing
- **WHEN** a MathML equation contains relational, arrow, or set-difference operators (`≤`, `≥`, `≠`, `→`, `←`, `⇒`, `⇔`, `∖`, or literal `\` set-minus)
- **THEN** `to_latex` emits `\leq`, `\geq`, `\neq`, `\rightarrow`, `\leftarrow`, `\Rightarrow`, `\Leftrightarrow`, and `\setminus` with consistent surrounding operator spacing (for example `A \setminus B` and `x \leq 0`)

#### Scenario: Standard spaces, style declarations, primes, and double bars serialize to standard TeX/LaTeX
- **WHEN** a MathML equation contains standard-width `<mspace>` elements, `mstyle` display/text style wrappers, prime marks up to triple prime (or legacy `⁗`), or nested double overbars/underbars
- **THEN** `to_latex` emits `\,`, `\:`, `\;`, `\!`, `\enspace`, `\quad`, `\qquad` (reserving `\hspace{...}` for non-standard widths), `{\displaystyle ...}` / `{\textstyle ...}`, `'`, `''`, `'''` (or `^{(4)}` for legacy `⁗`), and `\overline{\overline{...}}` / `\underline{\underline{...}}`

#### Scenario: Stacked fractions without bar serialize to standard LaTeX
- **WHEN** a MathML equation contains a fraction with zero line thickness (`linethickness="0"`) inside parentheses or bare
- **THEN** `to_latex` emits `\binom{n}{k}` when enclosed by parentheses and `{n \atop k}` when bare rather than `\atopfrac{n}{k}`

#### Scenario: Greek variants and named functions preserve distinctions and standard upright markup
- **WHEN** a MathML equation contains Greek variant identifiers (`ϵ`/`ε`, `ϕ`/`φ`, `θ`/`ϑ`, `π`/`ϖ`, `ρ`/`ϱ`, `σ`/`ς`), Latin-identical uppercase Greek glyphs, or upright named functions (`limsup`, `liminf`, `lg`, `sgn`, `lcm`, `arccot`)
- **THEN** `to_latex` distinguishes `\epsilon`/`\varepsilon`, `\phi`/`\varphi`, `\theta`/`\vartheta`, `\pi`/`\varpi`, `\rho`/`\varrho`, `\sigma`/`\varsigma`, avoids non-standard `\Alpha`–`\Chi` or `\omicron`, emits standard LaTeX function macros (`\limsup`, `\liminf`, `\lg`), and uses `\operatorname{...}` for non-builtin functions such as `\operatorname{sgn}`

### Requirement: Permissive LaTeX input acceptance for standard aliases and legacy extensions
When parsing LaTeX input from the LaTeX source box, Equation Canvas command input, paste, or conversion APIs, the parser SHALL accept both short and explicit standard LaTeX aliases as well as legacy custom forms.

#### Scenario: Short, explicit, and legacy aliases parse to equivalent MathML
- **WHEN** the user enters `\le` or `\leq`, `\ge` or `\geq`, `\ne` or `\neq`, `\to` or `\rightarrow`, `\gets` or `\leftarrow`, `\implies` or `\Rightarrow`, `\iff` or `\Leftrightarrow`, `\land` or `\wedge`, `\lor` or `\vee`, `\lnot` or `\neg`, `\dots` or `\ldots`, `\smallsetminus`, `\varnothing`, `\lvert`/`\rvert`/`\lVert`/`\rVert`, `{\textstyle ...}` or `\textstyle{...}`, `\binom{n}{k}` or `{n \atop k}` or `\atopfrac{n}{k}`, or `\overline{\overline{x}}` or `\doubleoverline{x}`
- **THEN** the parser succeeds and produces equivalent MathML trees

### Requirement: Canonical internal normalization for set-minus and degree across MathML, OMML, and MTEF
When parsing LaTeX or decoding OMML/MTEF equations, equivalent representations of set-minus and degree SHALL converge to one canonical internal MathML, OMML, and MTEF/OLE representation and serialize back cleanly to standard explicit LaTeX.

#### Scenario: Degree inputs unify to one canonical representation and serialize as `^\circ`
- **WHEN** an equation enters via LaTeX as `90^\circ`, `90^{\circ}`, `90\degree`, or `90°`, or enters from OMML/MTEF encoding `°` (`U+00B0`)
- **THEN** all three produce the same canonical MathML, OMML, and MTEF/OLE output, and `to_latex` serializes the degree marker as `^\circ` while inline `\circ` (`∘`, `U+2218`) remains distinct

#### Scenario: Set-minus from LaTeX, OMML, or MTEF roundtrips as `\setminus`
- **WHEN** an equation containing set-minus enters via LaTeX (`\setminus`), OMML (`<m:t>∖</m:t>`), or MathType MTEF (`0x005C` / `0x2216`)
- **THEN** OMML/MTEF decode treats set-minus as an operator rather than an identifier or raw backslash escape, and `to_latex` serializes it as `\setminus`

### Requirement: Retention of MathML/OMML-only and custom extension commands
Commands representing MathML, OMML, or MathType constructs without a standard LaTeX equivalent SHALL remain parseable, serializable, and selectable in the desktop command catalog.

#### Scenario: MathML/OMML-only constructs survive LaTeX roundtrip
- **WHEN** an equation uses `\enclose{longdiv}{...}`, `\enclose{roundedbox}{...}`, `\enclose{top left}{...}`, `\tsfrac{...}{...}`, `\sfrac{...}{...}`, `\prescript{...}{...}{...}`, `\overaccent{...}{...}`, `\underaccent{...}{...}`, directional harpoon/under-accents, or MathType-specific symbols without standard LaTeX equivalents
- **THEN** parsing and `to_latex` roundtrip preserve the structural semantics and attributes without error

### Requirement: Standard-aligned desktop command suggestions and palette templates
The desktop LaTeX command popup and palette templates SHALL surface standard explicit LaTeX forms for standardized constructs while keeping MathML/OMML-only extensions accessible.

#### Scenario: Command popup and palette prefer standard explicit LaTeX
- **WHEN** the user opens command suggestions or inserts a palette template for primes, double bars, standard spaces, or binomials
- **THEN** the command catalog provides standard forms (`\,`, `\quad`, `\binom{#?}{#?}`, `\overline{\overline{#?}}`) and omits quadruple-prime (`\qprime`, `{#?}^{⁗}`) rather than emitting custom replacements (`\hspace{0.166667em}`, `\atopfrac`, `\doubleoverline`)
