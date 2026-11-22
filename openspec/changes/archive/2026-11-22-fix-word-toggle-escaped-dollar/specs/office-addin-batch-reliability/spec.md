# Spec Delta

## ADDED Requirements

### Requirement: Word Toggle LaTeX preserves escaped dollars

Word Toggle LaTeX SHALL treat a dollar preceded by an odd number of consecutive backslashes as literal content when detecting formula delimiters. An even number of consecutive backslashes SHALL leave the dollar eligible to act as a delimiter.

#### Scenario: Literal dollar before a formula

- **WHEN** the user selects prose containing `Giá \$5, công thức $x+1$` and invokes Toggle LaTeX
- **THEN** Word converts only the actual formula and preserves the preceding prose

#### Scenario: Literal dollar inside a formula

- **WHEN** the user selects or places the caret inside a delimited formula containing `\$` and invokes Toggle LaTeX
- **THEN** Word recognizes the complete formula and passes the escaped dollar through to conversion

#### Scenario: Consecutive formulas and escaped dollars

- **WHEN** a selection contains multiple valid formulas separated by text with escaped dollars
- **THEN** Word converts each valid formula once and preserves the text between them
