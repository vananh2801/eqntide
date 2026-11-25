# Spec Delta

## ADDED Requirements

### Requirement: Transparent dashed placeholders and lightly rounded canvas overlays
Within the Equation Canvas, empty structural placeholder boxes (`□`) SHALL render with a dashed border using the active `CanvasPalette` placeholder stroke color, light `2px` corner rounding, and `0` inner fill opacity (`Color32::TRANSPARENT` over the canvas background), and selection highlights SHALL also use `2px` corner rounding.

#### Scenario: Empty structural placeholder displays an unfilled dashed box with light corner rounding
- **WHEN** an empty structural slot (`□`) is rendered on the Equation Canvas under any Light or Dark mode `CanvasPreset`
- **THEN** the slot displays a dashed outline with `2px` corner rounding and no tinted background fill inside the box (`placeholder_bg` alpha is `0`), revealing the active canvas background color inside the dashed border
