## ADDED Requirements

### Requirement: Complete named mathematical function parity across LaTeX, MathML, OMML, and MTEF
All named mathematical functions surfaced in the desktop `Functions` catalog (trigonometric, inverse trigonometric, hyperbolic, inverse hyperbolic, regional trigonometric aliases, logarithmic/exponential, limit/extremum, and algebraic operators) SHALL parse cleanly from LaTeX, render as upright mathematical function operators, serialize to canonical explicit LaTeX, and preserve upright function semantics across MathML, OMML, and MTEF/OLE conversions.

#### Scenario: Standard and operatorname functions parse, render upright, and serialize canonically
- **WHEN** an equation uses any supported named function—including standard LaTeX macros (`\sin`, `\cos`, `\tan`, `\cot`, `\sec`, `\csc`, `\arcsin`, `\arccos`, `\arctan`, `\sinh`, `\cosh`, `\tanh`, `\coth`, `\log`, `\ln`, `\lg`, `\exp`, `\lim`, `\limsup`, `\liminf`, `\min`, `\max`, `\sup`, `\inf`, `\det`, `\dim`, `\ker`, `\deg`, `\arg`, `\gcd`, `\Pr`) or `\operatorname{...}` functions (`arccot`, `arcsec`, `arccsc`, `sech`, `csch`, `arsinh`, `arcosh`, `artanh`, `arcoth`, `tg`, `ctg`, `cotg`, `cosec`, `hom`, `lcm`, `sgn`, `mod`)
- **THEN** LaTeX parsing produces upright function MathML without parse errors, `to_latex` serializes standard macros directly and non-builtin names via `\operatorname{...}`, and OMML and MTEF/OLE roundtrips retain upright function formatting

#### Scenario: Spaced and unspaced limit superior/inferior unify across renderer and encoders
- **WHEN** `\limsup` or `\liminf` is parsed to MathML or imported with `"lim sup"` / `"lim inf"` / `"limsup"` / `"liminf"` text
- **THEN** layout rendering, OMML encoding, MTEF encoding, and `to_latex` all treat both spaced and unspaced forms consistently as upright limit operators
