## MODIFIED Requirements

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

## ADDED Requirements

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
