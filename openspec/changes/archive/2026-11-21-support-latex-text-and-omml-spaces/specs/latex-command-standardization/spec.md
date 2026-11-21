# Spec Delta

## ADDED Requirements

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
The desktop command catalog SHALL offer `\text`. The canvas SHALL insert text from this command only when the user presses Enter with a complete braced argument.

#### Scenario: A complete argument remains buffered until Enter
- **WHEN** the user types `\text{5 cm }` on the desktop canvas
- **THEN** the command remains editable in the buffer after `}`, and Enter inserts a text span with its spaces preserved and places the caret after it in Math mode

#### Scenario: An incomplete text command is invalid
- **WHEN** the user presses Enter after `\text` or with an unfinished text argument
- **THEN** the canvas reports an invalid LaTeX command and retains the buffer for correction. The equation stays unchanged

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
