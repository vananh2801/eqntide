# latex-command-standardization Specification

## Purpose

Defines canonical, standard-aligned LaTeX parsing, normalization across MathML/OMML/MTEF representations, and explicit LaTeX serialization while retaining MathML/OMML-only and legacy extension commands.

## Requirements

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

### Requirement: Complete named mathematical function parity across LaTeX, MathML, OMML, and MTEF
All named mathematical functions surfaced in the desktop `Functions` catalog (trigonometric, inverse trigonometric, hyperbolic, inverse hyperbolic, regional trigonometric aliases, logarithmic/exponential, limit/extremum, and algebraic operators) SHALL parse cleanly from LaTeX, render as upright mathematical function operators, serialize to canonical explicit LaTeX, and preserve upright function semantics across MathML, OMML, and MTEF/OLE conversions.

#### Scenario: Standard and operatorname functions parse, render upright, and serialize canonically
- **WHEN** an equation uses any supported named function—including standard LaTeX macros (`\sin`, `\cos`, `\tan`, `\cot`, `\sec`, `\csc`, `\arcsin`, `\arccos`, `\arctan`, `\sinh`, `\cosh`, `\tanh`, `\coth`, `\log`, `\ln`, `\lg`, `\exp`, `\lim`, `\limsup`, `\liminf`, `\min`, `\max`, `\sup`, `\inf`, `\det`, `\dim`, `\ker`, `\deg`, `\arg`, `\gcd`, `\Pr`) or `\operatorname{...}` functions (`arccot`, `arcsec`, `arccsc`, `sech`, `csch`, `arsinh`, `arcosh`, `artanh`, `arcoth`, `tg`, `ctg`, `cotg`, `cosec`, `hom`, `lcm`, `sgn`, `mod`)
- **THEN** LaTeX parsing produces upright function MathML without parse errors, `to_latex` serializes standard macros directly and non-builtin names via `\operatorname{...}`, and OMML and MTEF/OLE roundtrips retain upright function formatting

#### Scenario: Spaced and unspaced limit superior/inferior unify across renderer and encoders
- **WHEN** `\limsup` or `\liminf` is parsed to MathML or imported with `"lim sup"` / `"lim inf"` / `"limsup"` / `"liminf"` text
- **THEN** layout rendering, OMML encoding, MTEF encoding, and `to_latex` all treat both spaced and unspaced forms consistently as upright limit operators

### Requirement: Full native `\text{...}` parsing and space-preserving serialization
When parsing LaTeX input containing `\text{...}`, the parser SHALL preserve all spaces (leading, interior, trailing, and space-only), digits, punctuation, Unicode/Vietnamese characters, and standard LaTeX text escapes into `<mtext>`, and `to_latex` SHALL serialize `<mtext>` without stripping boundary spaces or collapsing space-only runs.

#### Scenario: Text with spaces, digits, punctuation, and Vietnamese parses to `<mtext>` and converts to MathType
- **WHEN** LaTeX input contains `\text{hello world}`, `\text{5 cm}`, `\text{m/s}`, `\text{với }x\text{ (khi }x > 0\text{)}`, or `\text{ }`
- **THEN** parsing succeeds without truncation or parse error, produces `<mtext>` with exact text and whitespace intact, and converts cleanly to MathType OLE/MTEF, OMML, and EMF/PNG previews

#### Scenario: Escaped LaTeX text punctuation roundtrips cleanly
- **WHEN** LaTeX input contains `\text{a\_b \& \{c\} \% \$ \# \textbackslash{}}`
- **THEN** parsing unescapes those sequences into literal text inside `<mtext>` and `to_latex` serializes them back as valid `\text{...}` without losing boundary spaces

### Requirement: Continuous LaTeX serialization of adjacent text
The serializer SHALL combine consecutive MathML text siblings into one `\text{...}` span, preserve their spaces, and normalize nonbreaking spaces to ordinary spaces in LaTeX output.

#### Scenario: MathType text characters recombine after a roundtrip
- **WHEN** `\text{ khi}x\text{ với }` is converted to MathType and back to LaTeX
- **THEN** the output is `\text{ khi}x\text{ với }` with one text argument on each side of the math variable `x`

### Requirement: Confirmation of braced text commands on the desktop canvas
The desktop command catalog SHALL offer `\text` and all supported non-structural braced commands (`\mathbb`, `\mathfrak`, `\mathcal`, `\mathscr`, `\mathrm`, `\mathbf`, `\mathit`, `\boldsymbol`, `\operatorname`, `\hspace`, `\textstyle`, `\displaystyle`, `\enclose`). The canvas SHALL insert from these commands only when the user presses `Enter` with complete, non-empty, and valid braced arguments.

#### Scenario: A complete argument remains buffered until Enter
- **WHEN** the user types `\text{5 cm }` or `\mathbb{R}` on the desktop canvas
- **THEN** the command remains editable in the buffer after `}`, and `Enter` inserts the parsed MathML node and places the caret after it in Math mode

#### Scenario: An incomplete text command is invalid
- **WHEN** the user presses `Enter` after `\text` or with an unfinished or empty text argument (`\text{}`)
- **THEN** the canvas reports an invalid LaTeX command in the bottom status bar and retains the buffer for correction while leaving the equation unchanged

#### Scenario: An incomplete or empty braced font, style, spacing, operator, or enclosure command is invalid
- **WHEN** the user presses `Enter` after bare `\mathbb`, `\mathfrak`, `\mathcal`, `\mathscr`, `\mathrm`, `\mathbf`, `\mathit`, `\boldsymbol`, `\operatorname`, `\hspace`, `\textstyle`, `\displaystyle`, `\enclose`, with an unfinished argument, or with empty braces (`{}`)
- **THEN** the canvas reports an invalid LaTeX command in the bottom status bar and retains the buffer for correction while leaving the equation unchanged

### Requirement: Text and Math caret stops on the desktop canvas
The canvas SHALL distinguish the Text insertion point just inside each edge of a text span from the Math insertion point just outside it. The caret SHALL use the selected canvas palette's color for the active mode. Text mode SHALL accept literal input, and typed Space SHALL insert a character only in Text mode.

#### Scenario: Arrows switch modes at either edge
- **WHEN** the user arrows outward from a Text endpoint or inward from its adjacent Math insertion point
- **THEN** the caret switches mode at the same visual position and immediately uses the new mode color. Arrows within Text move across a visible character, including a decomposed Vietnamese letter

#### Scenario: Text mode accepts spaces and literal shortcut characters
- **WHEN** the user types Space or a backslash inside a text span
- **THEN** the character is inserted as text and no LaTeX command popup opens. Space at an adjacent Math insertion point leaves the equation unchanged

#### Scenario: Enter exits an existing text span
- **WHEN** the user presses Enter while editing an existing text span on the canvas
- **THEN** the caret moves after the span in Math mode

#### Scenario: The caret color uses the active canvas preset
- **WHEN** the canvas uses a light or dark preset selected in Settings and the caret switches modes
- **THEN** the caret is blue in Math and amber in Text using the active preset colors, with no text-span underline

### Requirement: Alphabet and syntax validation for specialized math-font commands in desktop authoring
When authoring LaTeX on the desktop application, specialized mathematical alphabet commands (`\mathbb`, `\mathcal`, `\mathscr`, `\mathfrak`) SHALL validate that their braced argument contains only characters supported by both the desktop renderer and MathType/OMML exports without nested script or structural markup.

#### Scenario: Supported alphabet characters pass validation
- **WHEN** desktop validation inspects `\mathbb{R}`, `\mathbb{Z}`, `\mathbb{k}`, `\mathcal{A}`, `\mathcal{L}`, `\mathscr{A}`, or `\mathfrak{Ag}`
- **THEN** validation succeeds (`\mathbb` allows uppercase `A–Z` and lowercase `k`; `\mathcal` allows uppercase `A–Z` and lowercase `l`; `\mathscr` allows `A–Z` and `a–z`; `\mathfrak` allows `A–Z` and `a–z`)

#### Scenario: Unsupported digits, operators, or nested scripts fail desktop validation
- **WHEN** desktop validation inspects `\mathbb{1^2}`, `\mathbb{1}`, `\mathbb{a}`, `\mathcal{1}`, `\mathfrak{1}`, or `\mathbb{R^2}`
- **THEN** validation fails with a specific diagnostic message and blocks committing or saving from the desktop editor, while batch/document conversion APIs retain permissive parsing for legacy external documents

### Requirement: Grouped serialization of styled MathML expressions
When converting MathML with `mathvariant` attributes back to LaTeX, uniform styled subtrees and consecutive styled sibling nodes SHALL be emitted inside a single enclosing style command rather than wrapping each token separately, and `script` runs containing lowercase letters other than `l` SHALL emit `\mathscr`.

#### Scenario: Compound styled expression serializes without per-token repetition
- **WHEN** `mathml_to_latex` serializes parsed MathML for `\mathbf{x^2 + 2x - 1 = 0}`, `\boldsymbol{x^2 + 2x - 1 = 0}`, `\mathrm{H_2O + m/s^2}`, `\mathit{x^2 + 2x}`, `\mathbb{RZ}`, `\mathfrak{Ag}`, `\mathcal{AL}`, or `\mathscr{Aa}`
- **THEN** each expression is emitted with a single outer style command (`\mathbf{x^{2}+2x-1=0}`, `\boldsymbol{x^{2}+2x-1=0}`, `\mathrm{H_{2}O+m/s^{2}}`, `\mathit{x^{2}+2x}`, `\mathbb{RZ}`, `\mathfrak{Ag}`, `\mathcal{AL}`, `\mathscr{Aa}`) that roundtrips cleanly through desktop LaTeX validation
