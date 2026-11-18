# mtef-decode-fidelity Specification

## Purpose

Ensures MTEF v3 (Equation Editor 3.0) and MTEF v5 (MathType) records decode without character loss, stream desynchronization, spacing-as-arrow corruption, or directional/accent loss.

## Requirements

### Requirement: Equation Editor 3.0 space typeface decoding
When decoding MTEF `REC_CHAR` records with typeface `24` (`fnSPACE` in Equation Editor 3.0 / MTEF v3), codes `0xEB01`..=`0xEB08` SHALL decode to spacing (`<mspace>` or whitespace `<mtext>`), never to Unicode arrow glyphs (`⥂`, `⥄`, `⤢`, `⤡`).

#### Scenario: Closed interval with thin spaces in Equation.3 decodes without arrows
- **WHEN** an `Equation.3` MTEF v3 binary encodes `[0 ; \pi/2]` using typeface `24` and code `0xEB02` around `;`
- **THEN** the decoded MathML and OMML contain spaces around `;` and do not contain `⥄` (`U+2944`)

### Requirement: Fallback character decoding when OPT_CHAR_NO_MT is set
When an MTEF `REC_CHAR` record has `OPT_CHAR_NO_MT` set and carries an `enc_8` or `enc_16` character code, the decoder SHALL map and preserve that character code using the record's typeface instead of discarding it and returning `None`.

#### Scenario: Character record with OPT_CHAR_NO_MT retains its glyph
- **WHEN** an MTEF v5 stream contains a `REC_CHAR` record with `OPT_CHAR_NO_MT` and `OPT_CHAR_ENC_8` set for an ASCII or Symbol byte
- **THEN** the decoded MathML includes the corresponding character node rather than dropping it

### Requirement: Nudge consumption on REC_EMBELL records
When an MTEF `REC_EMBELL` record has the `OPT_NUDGE` flag set in its options byte, the decoder SHALL consume the nudge payload before reading subsequent records.

#### Scenario: Nudged embellishment does not corrupt following MTEF records
- **WHEN** an MTEF stream contains a character followed by an `OPT_NUDGE`-flagged `REC_EMBELL` and trailing sibling characters
- **THEN** the trailing sibling characters decode intact without stream misalignment

### Requirement: Horizontal brace, under-accent, and directional arrow fidelity
MTEF decoding SHALL emit horizontal curly braces (`U+23DE`/`U+23DF`) for `tmHBRACE`, include `accentunder="true"` on under-accent templates 25–28, and preserve left vs right orientation for harpoons and paired arrows in `tmARROW`.

#### Scenario: Horizontal brace and directional harpoons preserve semantics
- **WHEN** MTEF records encode `tmHBRACE` over/under-braces, under-embellishments 25–28, and one-sided harpoons in `tmARROW`
- **THEN** the decoded MathML emits `U+23DE`/`U+23DF`, sets `accentunder="true"` on the under-accents, and distinguishes left-facing from right-facing harpoons/double-arrows
