# text-and-unit-space-fidelity Specification

## Purpose

Preserves spaces in natural-language text, unit spans, normal-text OMML runs, and standalone space runs across OMML, MathML, MathType MTEF/OLE, and rendered EMF/PNG previews without adding spurious spaces to math variables or numbers.

## Requirements

### Requirement: OMML normal-text and unit-span space preservation
When decoding an OMML run containing normal text, unit labels, or standalone spaces, the decoder SHALL preserve leading, interior, trailing, and space-only ASCII spaces (`U+0020`) and non-breaking spaces (`U+00A0`) as MathML `<mtext>` instead of trimming or discarding them.

#### Scenario: Unit or text run with leading or trailing space preserves spacing
- **WHEN** an OMML equation contains a plain-style or normal-text run such as `" cm"`, `"m "`, `"với "`, or `"khi x"`
- **THEN** decoding produces `<mtext>` retaining the boundary and interior spaces, and converting to MathType emits `TF_TEXT` space records (`0x20`) rather than collapsing adjacent tokens

#### Scenario: Standalone space run between number and unit is preserved
- **WHEN** an OMML equation contains a standalone space run (`" "`) between a number run and a unit run (for example `5` + `" "` + `cm`)
- **THEN** decoding preserves the space run as `<mtext> </mtext>` so MathType conversion and EMF preview retain the gap between `5` and `cm`

#### Scenario: Ordinary math-mode variables and numbers without explicit spaces remain unaffected
- **WHEN** an OMML equation contains ordinary math-mode runs such as `x`, `+`, `1`, or `ab` without normal-text or standalone-space runs
- **THEN** decoding continues to emit standard `<mi>`, `<mo>`, and `<mn>` tokens without introducing extra spaces

### Requirement: Normal-text marking on OMML encoding
When encoding a MathML `<mtext>` element into OMML, the encoder SHALL emit the OMML normal-text property alongside plain styling so short text, units, and space-only runs retain their text semantics across OMML roundtrips.

#### Scenario: Short and space-only text runs roundtrip through OMML as normal text
- **WHEN** a MathML equation containing `<mtext>cm</mtext>`, `<mtext>m </mtext>`, or `<mtext> </mtext>` is encoded to OMML and decoded back
- **THEN** the generated OMML run includes normal-text metadata and uses `xml:space="preserve"` on text with boundary whitespace. Decoding recovers `<mtext>` with its exact text and spaces

### Requirement: Space-only and single-letter-plus-space `<mtext>` fidelity in MTEF and rendering
When encoding `<mtext>` to MTEF or laying it out for EMF/PNG rendering, the system SHALL preserve space-only `<mtext>` elements and single-character units followed by spaces via text-font spacing rather than dropping spaces or reclassifying them as math operators.

#### Scenario: Space-only text survives MTEF encoding and EMF preview rendering
- **WHEN** an equation contains `<mtext> </mtext>` (from OMML, `\text{ }`, or Space typed in a canvas text span) and is converted to MathType OLE/MTEF with an EMF preview
- **THEN** MTEF contains a `TF_TEXT` space character record (`0x20`) and the rendered EMF layout advances horizontally by the text space width

#### Scenario: Single-letter unit with trailing space renders as text with space
- **WHEN** an equation contains `<mtext>m </mtext>` or `<mtext>s </mtext>`
- **THEN** layout rendering draws the unit in the text font and preserves the trailing space advance width rather than treating it as a trimmed symbol operator
