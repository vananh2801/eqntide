## ADDED Requirements

### Requirement: Extended OpenType MATH layout constants
`MathConstants` extracted from `STIXTwoMath-Regular.ttf` SHALL also expose OpenType `MATH` constants for script drops/bounds, large operator limit gaps and minimum heights, accent base height, overbar/underbar metrics, and skewed fraction gaps with matching `Default` fallbacks and without altering existing `MathConstants` values.

#### Scenario: Extended MATH constants match STIXTwoMath-Regular.ttf
- **WHEN** `fonts()` initializes `MathConstants` from the bundled `STIXTwoMath-Regular.ttf`
- **THEN** existing fields keep their exact prior values and newly added fields (`subscript_top_max`, `subscript_baseline_drop_min`, `superscript_bottom_min`, `superscript_baseline_drop_max`, `superscript_bottom_max_with_subscript`, `space_after_script`, `display_operator_min_height`, `upper_limit_gap_min`, `upper_limit_baseline_rise_min`, `lower_limit_gap_min`, `lower_limit_baseline_drop_min`, `accent_base_height`, `overbar_vertical_gap`, `overbar_rule_thickness`, `overbar_extra_ascender`, `underbar_vertical_gap`, `underbar_rule_thickness`, `underbar_extra_descender`, `skewed_fraction_horizontal_gap`, `skewed_fraction_vertical_gap`) match the `MATH` table normalized by `units_per_em`
