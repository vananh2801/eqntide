//! Shared MathType-style palette inventory and insertion templates.
//!
//! The desktop, canvas completion and LaTeX parser read this same catalog.
use crate::MathNode;

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub id: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct Symbol {
    pub name: &'static str,
    pub glyph: &'static str,
    pub tag: &'static str,
    pub group: &'static str,
    pub label: &'static str,
}

pub const GROUPS: &[(&str, &str)] = &[
    ("Fences", "Fences"),
    ("Fractions", "Fractions & Radicals"),
    ("Scripts", "Scripts"),
    ("Integrals", "Integrals"),
    ("Sums", "Sums"),
    ("BigOperators", "Products & Big Ops"),
    ("Matrices", "Matrices"),
    ("Embellishments", "Embellishments"),
    ("Bars", "Bars"),
    ("LabeledArrows", "Labeled Arrows"),
    ("Boxes", "Boxes"),
    ("Relations", "Relations"),
    ("Operators", "Operators"),
    ("Arrows", "Arrows"),
    ("Logic", "Logic"),
    ("Sets", "Sets"),
    ("Symbols", "Symbols"),
    ("GreekLower", "Lowercase Greek"),
    ("GreekUpper", "Uppercase Greek"),
    ("Spaces", "Spaces & Dots"),
];

macro_rules! catalog {
    (symbols: [$(($sg:literal, $name:literal, $glyph:literal, $label:literal, $tag:literal)),* $(,)?],
     templates: [$(($id:literal, $tg:literal, $tl:literal, $source:literal)),* $(,)?],
     commands: [$(($cn:literal, $cl:literal, $cs:literal)),* $(,)?]) => {
        pub const SYMBOLS: &[Symbol] = &[$(Symbol { name: $name, glyph: $glyph, tag: $tag, group: $sg, label: $label }),*];
        pub const ENTRIES: &[Entry] = &[
            $(Entry { id: concat!("symbol-", $name), group: $sg, label: $label, source: concat!("\\", $name) }),*,
            $(Entry { id: concat!($tg, "-", $id), group: $tg, label: $tl, source: $source }),*
        ];
        pub const PALETTE: &[(&str, &str, &str)] = &[
            $(($sg, $label, concat!("\\", $name))),*,
            $(($tg, $tl, $source)),*
        ];
        pub const COMMANDS: &[(&str, &str, &str)] = &[
            $(($name, $label, concat!("\\", $name))),*,
            $(($cn, $cl, $cs)),*
        ];
    };
}

catalog! {
    symbols: [
        ("Relations", "le", "≤", "Less than or equal to", "mo"),
        ("Relations", "ge", "≥", "Greater than or equal to", "mo"),
        ("Relations", "ll", "≪", "Much less than", "mo"),
        ("Relations", "gg", "≫", "Much greater than", "mo"),
        ("Relations", "prec", "≺", "Precedes", "mo"),
        ("Relations", "succ", "≻", "Succeeds", "mo"),
        ("Relations", "triangleleft", "◁", "Normal subgroup of", "mo"),
        ("Relations", "triangleright", "▷", "Contains as normal subgroup", "mo"),
        ("Relations", "sim", "∼", "Similar to", "mo"),
        ("Relations", "approx", "≈", "Approximately equal to", "mo"),
        ("Relations", "simeq", "≃", "Asymptotically equal to", "mo"),
        ("Relations", "cong", "≅", "Congruent to", "mo"),
        ("Relations", "neq", "≠", "Not equal to", "mo"),
        ("Relations", "equiv", "≡", "Identical to", "mo"),
        ("Relations", "doteq", "≐", "Equals by definition", "mo"),
        ("Relations", "triangleq", "≜", "Defined as", "mo"),
        ("Relations", "estimates", "≙", "Estimates", "mo"),
        ("Relations", "fallingdotseq", "≒", "Approximately equal or image of", "mo"),
        ("Relations", "risingdotseq", "≓", "Image of or approximately equal", "mo"),
        ("Relations", "asymp", "≍", "Equivalent to", "mo"),
        ("Relations", "propto", "∝", "Proportional to", "mo"),
        ("Relations", "nleq", "≰", "Not less than or equal to", "mo"),
        ("Relations", "ngeq", "≱", "Not greater than or equal to", "mo"),
        ("Relations", "preceq", "≼", "Precedes or equal to", "mo"),
        ("Relations", "succeq", "≽", "Succeeds or equal to", "mo"),
        ("Operators", "pm", "±", "Plus-minus", "mo"),
        ("Operators", "mp", "∓", "Minus-plus", "mo"),
        ("Operators", "times", "×", "Multiplication", "mo"),
        ("Operators", "ast", "∗", "Asterisk operator", "mo"),
        ("Operators", "div", "÷", "Division", "mo"),
        ("Operators", "oplus", "⊕", "Circled plus", "mo"),
        ("Operators", "otimes", "⊗", "Circled times", "mo"),
        ("Operators", "odot", "⊙", "Circled dot", "mo"),
        ("Operators", "cdot", "⋅", "Dot operator", "mo"),
        ("Operators", "bullet", "•", "Bullet operator", "mo"),
        ("Operators", "circ", "∘", "Ring operator", "mo"),
        ("Operators", "smallcirc", "○", "White circle", "mo"),
        ("Operators", "diamond", "⋄", "Diamond operator", "mo"),
        ("Operators", "dagger", "†", "Dagger", "mo"),
        ("Operators", "ddagger", "‡", "Double dagger", "mo"),
        ("Operators", "amalg", "⨿", "Coproduct", "mo"),
        ("Operators", "langle", "⟨", "Left angle bracket", "mo"),
        ("Operators", "rangle", "⟩", "Right angle bracket", "mo"),
        ("Operators", "llbracket", "⟦", "Left double bracket", "mo"),
        ("Operators", "rrbracket", "⟧", "Right double bracket", "mo"),
        ("Operators", "Vert", "‖", "Double vertical bar", "mo"),
        ("Operators", "parallel", "∥", "Parallel to", "mo"),
        ("Operators", "perp", "⊥", "Perpendicular to", "mo"),
        ("Arrows", "leftrightarrow", "↔", "Left-right arrow", "mo"),
        ("Arrows", "rightarrow", "→", "Right arrow", "mo"),
        ("Arrows", "leftarrow", "←", "Left arrow", "mo"),
        ("Arrows", "updownarrow", "↕", "Up-down arrow", "mo"),
        ("Arrows", "uparrow", "↑", "Up arrow", "mo"),
        ("Arrows", "downarrow", "↓", "Down arrow", "mo"),
        ("Arrows", "Leftrightarrow", "⇔", "Double left-right arrow", "mo"),
        ("Arrows", "Rightarrow", "⇒", "Implies", "mo"),
        ("Arrows", "Leftarrow", "⇐", "Implied by", "mo"),
        ("Arrows", "Updownarrow", "⇕", "Double up-down arrow", "mo"),
        ("Arrows", "Uparrow", "⇑", "Double up arrow", "mo"),
        ("Arrows", "Downarrow", "⇓", "Double down arrow", "mo"),
        ("Arrows", "nearrow", "↗", "North east arrow", "mo"),
        ("Arrows", "searrow", "↘", "South east arrow", "mo"),
        ("Arrows", "swarrow", "↙", "South west arrow", "mo"),
        ("Arrows", "nwarrow", "↖", "North west arrow", "mo"),
        ("Arrows", "neswarrow", "⤢", "North east and south west arrow", "mo"),
        ("Arrows", "nwsearrow", "⤡", "North west and south east arrow", "mo"),
        ("Arrows", "leftrightarrows", "⇆", "Left over right arrows", "mo"),
        ("Arrows", "rightleftarrows", "⇄", "Right over left arrows", "mo"),
        ("Arrows", "rightleftharpoons", "⇌", "Right over left harpoons", "mo"),
        ("Arrows", "leftrightharpoons", "⇋", "Left over right harpoons", "mo"),
        ("Arrows", "rightharpoonup", "⇀", "Rightwards harpoon barb up", "mo"),
        ("Arrows", "rightharpoondown", "⇁", "Rightwards harpoon barb down", "mo"),
        ("Arrows", "leftharpoonup", "↼", "Leftwards harpoon barb up", "mo"),
        ("Arrows", "leftharpoondown", "↽", "Leftwards harpoon barb down", "mo"),
        ("Arrows", "mapsto", "↦", "Maps to", "mo"),
        ("Arrows", "hookleftarrow", "↩", "Leftwards hook arrow", "mo"),
        ("Arrows", "hookrightarrow", "↪", "Rightwards hook arrow", "mo"),
        ("Arrows", "longrightarrow", "⟶", "Long right arrow", "mo"),
        ("Arrows", "longleftarrow", "⟵", "Long left arrow", "mo"),
        ("Arrows", "longleftrightarrow", "⟷", "Long left-right arrow", "mo"),
        ("Arrows", "Longrightarrow", "⟹", "Long double right arrow", "mo"),
        ("Arrows", "Longleftarrow", "⟸", "Long double left arrow", "mo"),
        ("Arrows", "Longleftrightarrow", "⟺", "Long double left-right arrow", "mo"),
        ("Arrows", "curvearrowleft", "↶", "Anticlockwise top semicircle arrow", "mo"),
        ("Arrows", "curvearrowright", "↷", "Clockwise top semicircle arrow", "mo"),
        ("Arrows", "carriagereturn", "↵", "Carriage return arrow", "mo"),
        ("Logic", "therefore", "∴", "Therefore", "mo"),
        ("Logic", "because", "∵", "Because", "mo"),
        ("Logic", "exists", "∃", "There exists", "mo"),
        ("Logic", "backepsilon", "∍", "Such that", "mo"),
        ("Logic", "nexists", "∄", "There does not exist", "mo"),
        ("Logic", "forall", "∀", "For all", "mo"),
        ("Logic", "neg", "¬", "Logical not", "mo"),
        ("Logic", "land", "∧", "Logical and", "mo"),
        ("Logic", "lor", "∨", "Logical or", "mo"),
        ("Logic", "top", "⊤", "Tautology (top)", "mo"),
        ("Logic", "bot", "⊥", "Contradiction (bottom)", "mo"),
        ("Logic", "vdash", "⊢", "Proves (turnstile)", "mo"),
        ("Logic", "dashv", "⊣", "Left tack", "mo"),
        ("Logic", "models", "⊨", "Models", "mo"),
        ("Sets", "in", "∈", "Element of", "mo"),
        ("Sets", "notin", "∉", "Not an element of", "mo"),
        ("Sets", "ni", "∋", "Contains as member", "mo"),
        ("Sets", "notni", "∌", "Does not contain as member", "mo"),
        ("Sets", "cup", "∪", "Union", "mo"),
        ("Sets", "cap", "∩", "Intersection", "mo"),
        ("Sets", "subset", "⊂", "Subset of", "mo"),
        ("Sets", "supset", "⊃", "Superset of", "mo"),
        ("Sets", "nsubset", "⊄", "Not a subset of", "mo"),
        ("Sets", "subseteq", "⊆", "Subset of or equal to", "mo"),
        ("Sets", "supseteq", "⊇", "Superset of or equal to", "mo"),
        ("Sets", "nsubseteq", "⊈", "Not a subset of or equal to", "mo"),
        ("Sets", "nsupseteq", "⊉", "Not a superset of or equal to", "mo"),
        ("Sets", "emptyset", "∅", "Empty set", "mo"),
        ("Sets", "setminus", "∖", "Set minus", "mo"),
        ("Sets", "subsetneq", "⊊", "Proper subset of", "mo"),
        ("Sets", "supsetneq", "⊋", "Proper superset of", "mo"),
        ("Symbols", "partial", "∂", "Partial differential", "mo"),
        ("Symbols", "ell", "ℓ", "Script small l", "mi"),
        ("Symbols", "wp", "℘", "Weierstrass p", "mi"),
        ("Symbols", "Re", "ℜ", "Real part", "mi"),
        ("Symbols", "Im", "ℑ", "Imaginary part", "mi"),
        ("Symbols", "aleph", "ℵ", "Aleph", "mi"),
        ("Symbols", "infty", "∞", "Infinity", "mo"),
        ("Symbols", "hbar", "ℏ", "Reduced Planck constant", "mi"),
        ("Symbols", "lambdabar", "ƛ", "Lambda bar", "mi"),
        ("Symbols", "imath", "ı", "Dotless i", "mi"),
        ("Symbols", "jmath", "ȷ", "Dotless j", "mi"),
        ("Symbols", "nabla", "∇", "Nabla", "mo"),
        ("Symbols", "triangle", "△", "Triangle", "mo"),
        ("Symbols", "mho", "℧", "Mho", "mo"),
        ("Symbols", "angle", "∠", "Angle", "mo"),
        ("Symbols", "measuredangle", "∡", "Measured angle", "mo"),
        ("Symbols", "sphericalangle", "∢", "Spherical angle", "mo"),
        ("Symbols", "square", "□", "Square", "mo"),
        ("Symbols", "Box", "□", "Box", "mo"),
        ("Symbols", "rectangle", "▭", "Rectangle", "mo"),
        ("Symbols", "parallelogram", "▱", "Parallelogram", "mo"),
        ("Symbols", "bigcirc", "◯", "Large circle", "mo"),
        ("Symbols", "lozenge", "◊", "Lozenge", "mo"),
        ("Symbols", "degree", "°", "Degree", "mo"),
        ("Symbols", "prime", "′", "Prime", "mo"),
        ("Symbols", "backprime", "‵", "Reversed prime", "mo"),
        ("Symbols", "clubsuit", "♣", "Club suit", "mo"),
        ("Symbols", "heartsuit", "♡", "Heart suit", "mo"),
        ("Symbols", "diamondsuit", "♢", "Diamond suit", "mo"),
        ("Symbols", "spadesuit", "♠", "Spade suit", "mo"),
        ("GreekLower", "alpha", "α", "Greek alpha", "mi"),
        ("GreekLower", "beta", "β", "Greek beta", "mi"),
        ("GreekLower", "gamma", "γ", "Greek gamma", "mi"),
        ("GreekLower", "delta", "δ", "Greek delta", "mi"),
        ("GreekLower", "epsilon", "ϵ", "Greek epsilon", "mi"),
        ("GreekLower", "phi", "ϕ", "Greek phi", "mi"),
        ("GreekLower", "varphi", "φ", "Greek varphi", "mi"),
        ("GreekLower", "vartheta", "ϑ", "Greek vartheta", "mi"),
        ("GreekLower", "eta", "η", "Greek eta", "mi"),
        ("GreekLower", "iota", "ι", "Greek iota", "mi"),
        ("GreekLower", "kappa", "κ", "Greek kappa", "mi"),
        ("GreekLower", "lambda", "λ", "Greek lambda", "mi"),
        ("GreekLower", "mu", "μ", "Greek mu", "mi"),
        ("GreekLower", "nu", "ν", "Greek nu", "mi"),
        ("GreekLower", "omicron", "ο", "Greek omicron", "mi"),
        ("GreekLower", "pi", "π", "Greek pi", "mi"),
        ("GreekLower", "varpi", "ϖ", "Greek varpi", "mi"),
        ("GreekLower", "theta", "θ", "Greek theta", "mi"),
        ("GreekLower", "rho", "ρ", "Greek rho", "mi"),
        ("GreekLower", "varrho", "ϱ", "Greek varrho", "mi"),
        ("GreekLower", "sigma", "σ", "Greek sigma", "mi"),
        ("GreekLower", "varsigma", "ς", "Greek varsigma", "mi"),
        ("GreekLower", "tau", "τ", "Greek tau", "mi"),
        ("GreekLower", "upsilon", "υ", "Greek upsilon", "mi"),
        ("GreekLower", "omega", "ω", "Greek omega", "mi"),
        ("GreekLower", "xi", "ξ", "Greek xi", "mi"),
        ("GreekLower", "psi", "ψ", "Greek psi", "mi"),
        ("GreekLower", "zeta", "ζ", "Greek zeta", "mi"),
        ("GreekLower", "chi", "χ", "Greek chi", "mi"),
        ("GreekLower", "varepsilon", "ε", "Greek varepsilon", "mi"),
        ("GreekLower", "varkappa", "ϰ", "Greek varkappa", "mi"),
        ("GreekLower", "digamma", "ϝ", "Greek digamma", "mi"),
        ("GreekUpper", "Alpha", "Α", "Greek Alpha", "mi"),
        ("GreekUpper", "Beta", "Β", "Greek Beta", "mi"),
        ("GreekUpper", "Chi", "Χ", "Greek Chi", "mi"),
        ("GreekUpper", "Delta", "Δ", "Greek Delta", "mi"),
        ("GreekUpper", "Epsilon", "Ε", "Greek Epsilon", "mi"),
        ("GreekUpper", "Phi", "Φ", "Greek Phi", "mi"),
        ("GreekUpper", "Gamma", "Γ", "Greek Gamma", "mi"),
        ("GreekUpper", "Eta", "Η", "Greek Eta", "mi"),
        ("GreekUpper", "Iota", "Ι", "Greek Iota", "mi"),
        ("GreekUpper", "Kappa", "Κ", "Greek Kappa", "mi"),
        ("GreekUpper", "Lambda", "Λ", "Greek Lambda", "mi"),
        ("GreekUpper", "Mu", "Μ", "Greek Mu", "mi"),
        ("GreekUpper", "Nu", "Ν", "Greek Nu", "mi"),
        ("GreekUpper", "Omicron", "Ο", "Greek Omicron", "mi"),
        ("GreekUpper", "Pi", "Π", "Greek Pi", "mi"),
        ("GreekUpper", "Theta", "Θ", "Greek Theta", "mi"),
        ("GreekUpper", "Rho", "Ρ", "Greek Rho", "mi"),
        ("GreekUpper", "Sigma", "Σ", "Greek Sigma", "mi"),
        ("GreekUpper", "Tau", "Τ", "Greek Tau", "mi"),
        ("GreekUpper", "Upsilon", "Υ", "Greek Upsilon", "mi"),
        ("GreekUpper", "Omega", "Ω", "Greek Omega", "mi"),
        ("GreekUpper", "Xi", "Ξ", "Greek Xi", "mi"),
        ("GreekUpper", "Psi", "Ψ", "Greek Psi", "mi"),
        ("GreekUpper", "Zeta", "Ζ", "Greek Zeta", "mi"),
        ("GreekUpper", "Digamma", "Ϝ", "Greek Digamma", "mi"),
        ("GreekUpper", "varTheta", "ϴ", "Greek varTheta", "mi"),
        ("GreekUpper", "varUpsilon", "ϒ", "Greek varUpsilon", "mi"),
        ("Sums", "sum", "∑", "Summation", "mo"),
        ("Integrals", "int", "∫", "Integral", "mo"),
        ("Integrals", "iint", "∬", "Double integral", "mo"),
        ("Integrals", "iiint", "∭", "Triple integral", "mo"),
        ("Integrals", "oint", "∮", "Contour integral", "mo"),
        ("Integrals", "oiint", "∯", "Surface integral", "mo"),
        ("Integrals", "oiiint", "∰", "Volume integral", "mo"),
        ("Integrals", "varointclockwise", "∲", "Clockwise contour integral", "mo"),
        ("Integrals", "ointctrclockwise", "∳", "Anticlockwise contour integral", "mo"),
        ("BigOperators", "prod", "∏", "Product", "mo"),
        ("BigOperators", "coprod", "∐", "Coproduct", "mo"),
        ("BigOperators", "bigcap", "⋂", "Big intersection", "mo"),
        ("BigOperators", "bigcup", "⋃", "Big union", "mo"),
        ("BigOperators", "bigwedge", "⋀", "Big logical and", "mo"),
        ("BigOperators", "bigvee", "⋁", "Big logical or", "mo"),
        ("BigOperators", "bigotimes", "⨂", "Big tensor product", "mo"),
        ("BigOperators", "bigoplus", "⨁", "Big direct sum", "mo"),
        ("BigOperators", "bigodot", "⨀", "Big dot product", "mo"),
        ("Spaces", "ldots", "…", "Horizontal ellipsis", "mo"),
        ("Spaces", "cdots", "⋯", "Midline horizontal ellipsis", "mo"),
        ("Spaces", "vdots", "⋮", "Vertical ellipsis", "mo"),
        ("Spaces", "ddots", "⋱", "Down-right diagonal ellipsis", "mo"),
        ("Spaces", "iddots", "⋰", "Up-right diagonal ellipsis", "mo"),
        ("Spaces", "dprime", "″", "Double prime", "mo"),
        ("Spaces", "trprime", "‴", "Triple prime", "mo"),
        ("Spaces", "qprime", "⁗", "Quadruple prime", "mo"),
    ],
    templates: [
        ("number-set-R", "Symbols", "Real numbers R", "\\mathbb{R}"),
        ("number-set-Z", "Symbols", "Integers Z", "\\mathbb{Z}"),
        ("number-set-C", "Symbols", "Complex numbers C", "\\mathbb{C}"),
        ("number-set-Q", "Symbols", "Rationals Q", "\\mathbb{Q}"),
        ("number-set-N", "Symbols", "Naturals N", "\\mathbb{N}"),
        ("script-A", "Symbols", "Script A", "\\mathcal{A}"),
        ("script-B", "Symbols", "Script B", "\\mathcal{B}"),
        ("script-S", "Symbols", "Script S", "\\mathcal{S}"),
        ("script-R", "Symbols", "Script R", "\\mathcal{R}"),
        ("script-N", "Symbols", "Script N", "\\mathcal{N}"),
        ("fraktur-A", "Symbols", "Fraktur A", "\\mathfrak{A}"),
        ("fraktur-B", "Symbols", "Fraktur B", "\\mathfrak{B}"),
        ("fraktur-S", "Symbols", "Fraktur S", "\\mathfrak{S}"),
        ("fraktur-R", "Symbols", "Fraktur R", "\\mathfrak{R}"),
        ("fraktur-N", "Symbols", "Fraktur N", "\\mathfrak{N}"),
        ("paren-both", "Fences", "Parentheses", "\\left( #? \\right)"),
        ("paren-left", "Fences", "Left parenthesis", "\\left( #? \\right."),
        ("paren-right", "Fences", "Right parenthesis", "\\left. #? \\right)"),
        ("bracket-both", "Fences", "Square brackets", "\\left[ #? \\right]"),
        ("bracket-left", "Fences", "Left square bracket", "\\left[ #? \\right."),
        ("bracket-right", "Fences", "Right square bracket", "\\left. #? \\right]"),
        ("brace-both", "Fences", "Curly braces", "\\left\\{ #? \\right\\}"),
        ("brace-left", "Fences", "Left curly brace", "\\left\\{ #? \\right."),
        ("brace-right", "Fences", "Right curly brace", "\\left. #? \\right\\}"),
        ("angle-both", "Fences", "Angle brackets", "\\left\\langle #? \\right\\rangle"),
        ("angle-left", "Fences", "Left angle bracket", "\\left\\langle #? \\right."),
        ("angle-right", "Fences", "Right angle bracket", "\\left. #? \\right\\rangle"),
        ("bar-both", "Fences", "Absolute value", "\\left| #? \\right|"),
        ("bar-left", "Fences", "Left vertical bar", "\\left| #? \\right."),
        ("bar-right", "Fences", "Right vertical bar", "\\left. #? \\right|"),
        ("norm-both", "Fences", "Norm", "\\left\\Vert #? \\right\\Vert"),
        ("norm-left", "Fences", "Left double bar", "\\left\\Vert #? \\right."),
        ("norm-right", "Fences", "Right double bar", "\\left. #? \\right\\Vert"),
        ("floor-both", "Fences", "Floor", "\\left\\lfloor #? \\right\\rfloor"),
        ("floor-left", "Fences", "Left floor", "\\left\\lfloor #? \\right."),
        ("floor-right", "Fences", "Right floor", "\\left. #? \\right\\rfloor"),
        ("ceiling-both", "Fences", "Ceiling", "\\left\\lceil #? \\right\\rceil"),
        ("ceiling-left", "Fences", "Left ceiling", "\\left\\lceil #? \\right."),
        ("ceiling-right", "Fences", "Right ceiling", "\\left. #? \\right\\rceil"),
        ("bracket-paren", "Fences", "Half-open interval [ )", "\\left[ #? \\right)"),
        ("paren-bracket", "Fences", "Half-open interval ( ]", "\\left( #? \\right]"),
        ("ket", "Fences", "Ket", "\\left| #? \\right\\rangle"),
        ("bra", "Fences", "Bra", "\\left\\langle #? \\right|"),
        ("bracket-left-left", "Fences", "Square interval [ [", "\\left[ #? \\right["),
        ("bracket-right-right", "Fences", "Square interval ] ]", "\\left] #? \\right]"),
        ("bracket-open-open", "Fences", "Open square interval ] [", "\\left] #? \\right["),
        ("double-bracket-both", "Fences", "Double brackets", "\\left\\llbracket #? \\right\\rrbracket"),
        ("double-bracket-left", "Fences", "Left double bracket", "\\left\\llbracket #? \\right."),
        ("double-bracket-right", "Fences", "Right double bracket", "\\left. #? \\right\\rrbracket"),
        ("overbrace", "Fences", "Overbrace", "\\overbrace{#?}"),
        ("overbrace-label", "Fences", "Labeled overbrace", "\\overbrace{#?}^{#?}"),
        ("underbrace", "Fences", "Underbrace", "\\underbrace{#?}"),
        ("underbrace-label", "Fences", "Labeled underbrace", "\\underbrace{#?}_{#?}"),
        ("overbracket", "Fences", "Overbracket", "\\overbracket{#?}"),
        ("overbracket-label", "Fences", "Labeled overbracket", "\\overbracket{#?}^{#?}"),
        ("underbracket", "Fences", "Underbracket", "\\underbracket{#?}"),
        ("underbracket-label", "Fences", "Labeled underbracket", "\\underbracket{#?}_{#?}"),
        ("wideparen", "Fences", "Overarc", "\\wideparen{#?}"),
        ("underparen", "Fences", "Underarc", "\\underparen{#?}"),
        ("cases-two", "Fences", "Two-case piecewise (Ctrl+Shift+C)", "\\begin{cases} #? & #? \\\\ #? & #? \\end{cases}"),
        ("cases-three", "Fences", "Three-case piecewise", "\\begin{cases} #? & #? \\\\ #? & #? \\\\ #? & #? \\end{cases}"),
        ("evaluation", "Fences", "Evaluation bar", "\\left. #? \\right|_{#?}^{#?}"),
        ("bra-ket", "Fences", "Bra–ket", "\\left\\langle #? \\middle| #? \\right\\rangle"),
        ("frac", "Fractions", "Fraction (Ctrl+F or /)", "\\frac{#?}{#?}"),
        ("tfrac", "Fractions", "Small fraction", "\\tfrac{#?}{#?}"),
        ("dfrac", "Fractions", "Display fraction", "\\dfrac{#?}{#?}"),
        ("sfrac", "Fractions", "Bevelled fraction", "\\sfrac{#?}{#?}"),
        ("binom", "Fractions", "Binomial", "\\binom{#?}{#?}"),
        ("linear", "Fractions", "Inline fraction", "{#?}/{#?}"),
        ("small-linear", "Fractions", "Small inline fraction", "\\tsfrac{#?}{#?}"),
        ("sqrt", "Fractions", "Square root (Ctrl+R)", "\\sqrt{#?}"),
        ("root", "Fractions", "nth root (Ctrl+Shift+R)", "\\sqrt[#?]{#?}"),
        ("root-fraction", "Fractions", "Square root of fraction", "\\sqrt{\\frac{#?}{#?}}"),
        ("small-root", "Fractions", "Small square root", "\\sqrt{\\tfrac{#?}{#?}}"),
        ("long-division", "Fractions", "Long division", "\\enclose{longdiv}{#?}"),
        ("long-division-quotient", "Fractions", "Long division with quotient", "\\overset{#?}{\\enclose{longdiv}{#?}}"),
        ("sup", "Scripts", "Superscript (Ctrl+H or ^)", "^{#?}"),
        ("sub", "Scripts", "Subscript (Ctrl+L or _)", "_{#?}"),
        ("subsup", "Scripts", "Subscript and superscript (Ctrl+J)", "_{#?}^{#?}"),
        ("pre-sup-post-none", "Scripts", "Left superscript", "\\prescript{#?}{}{#?}"),
        ("pre-sup-post-sup", "Scripts", "Left superscript & right superscript", "\\prescript{#?}{}{#?}^{#?}"),
        ("pre-sup-post-sub", "Scripts", "Left superscript & right subscript", "\\prescript{#?}{}{#?}_{#?}"),
        ("pre-sup-post-both", "Scripts", "Left superscript & right sub/superscript", "\\prescript{#?}{}{#?}_{#?}^{#?}"),
        ("pre-sub-post-none", "Scripts", "Left subscript", "\\prescript{}{#?}{#?}"),
        ("pre-sub-post-sup", "Scripts", "Left subscript & right superscript", "\\prescript{}{#?}{#?}^{#?}"),
        ("pre-sub-post-sub", "Scripts", "Left subscript & right subscript", "\\prescript{}{#?}{#?}_{#?}"),
        ("pre-sub-post-both", "Scripts", "Left subscript & right sub/superscript", "\\prescript{}{#?}{#?}_{#?}^{#?}"),
        ("pre-both-post-none", "Scripts", "Left sub/superscript", "\\prescript{#?}{#?}{#?}"),
        ("pre-both-post-sup", "Scripts", "Left sub/superscript & right superscript", "\\prescript{#?}{#?}{#?}^{#?}"),
        ("pre-both-post-sub", "Scripts", "Left sub/superscript & right subscript", "\\prescript{#?}{#?}{#?}_{#?}"),
        ("pre-both-post-both", "Scripts", "Left sub/superscript & right sub/superscript", "\\prescript{#?}{#?}{#?}_{#?}^{#?}"),
        ("lim", "Scripts", "Limit (Ctrl+Shift+L)", "\\lim_{#? \\to #?}"),
        ("max", "Scripts", "Constrained maximum", "\\max_{#?}"),
        ("min", "Scripts", "Constrained minimum", "\\min_{#?}"),
        ("overset", "Scripts", "Overscript label", "\\overset{#?}{#?}"),
        ("underset", "Scripts", "Underscript label", "\\underset{#?}{#?}"),
        ("overunderset", "Scripts", "Overscript and underscript label", "\\overset{#?}{\\underset{#?}{#?}}"),
        ("sum-side-lower", "Sums", "Sum with side lower limit", "\\sum\\nolimits_{#?} #?"),
        ("sum-side-upper", "Sums", "Sum with side upper limit", "\\sum\\nolimits^{#?} #?"),
        ("sum-side-both", "Sums", "Sum with side limits", "\\sum\\nolimits_{#?}^{#?} #?"),
        ("sum-stack-lower", "Sums", "Sum with stacked lower limit", "\\sum\\limits_{#?} #?"),
        ("sum-stack-upper", "Sums", "Sum with stacked upper limit", "\\sum\\limits^{#?} #?"),
        ("sum-stack-both", "Sums", "Sum with stacked limits", "\\sum\\limits_{#?}^{#?} #?"),
        ("int-side-lower", "Integrals", "Integral with side lower limit", "\\int\\nolimits_{#?} #?"),
        ("int-side-upper", "Integrals", "Integral with side upper limit", "\\int\\nolimits^{#?} #?"),
        ("int-side-both", "Integrals", "Integral with side limits", "\\int\\nolimits_{#?}^{#?} #?"),
        ("int-stack-lower", "Integrals", "Integral with stacked lower limit", "\\int\\limits_{#?} #?"),
        ("int-stack-upper", "Integrals", "Integral with stacked upper limit", "\\int\\limits^{#?} #?"),
        ("int-stack-both", "Integrals", "Integral with stacked limits", "\\int\\limits_{#?}^{#?} #?"),
        ("iint-side-lower", "Integrals", "Double integral with side lower limit", "\\iint\\nolimits_{#?} #?"),
        ("iint-side-upper", "Integrals", "Double integral with side upper limit", "\\iint\\nolimits^{#?} #?"),
        ("iint-side-both", "Integrals", "Double integral with side limits", "\\iint\\nolimits_{#?}^{#?} #?"),
        ("iint-stack-lower", "Integrals", "Double integral with stacked lower limit", "\\iint\\limits_{#?} #?"),
        ("iint-stack-upper", "Integrals", "Double integral with stacked upper limit", "\\iint\\limits^{#?} #?"),
        ("iint-stack-both", "Integrals", "Double integral with stacked limits", "\\iint\\limits_{#?}^{#?} #?"),
        ("iiint-side-lower", "Integrals", "Triple integral with side lower limit", "\\iiint\\nolimits_{#?} #?"),
        ("iiint-side-upper", "Integrals", "Triple integral with side upper limit", "\\iiint\\nolimits^{#?} #?"),
        ("iiint-side-both", "Integrals", "Triple integral with side limits", "\\iiint\\nolimits_{#?}^{#?} #?"),
        ("iiint-stack-lower", "Integrals", "Triple integral with stacked lower limit", "\\iiint\\limits_{#?} #?"),
        ("iiint-stack-upper", "Integrals", "Triple integral with stacked upper limit", "\\iiint\\limits^{#?} #?"),
        ("iiint-stack-both", "Integrals", "Triple integral with stacked limits", "\\iiint\\limits_{#?}^{#?} #?"),
        ("oint-side-lower", "Integrals", "Contour integral with side lower limit", "\\oint\\nolimits_{#?} #?"),
        ("oint-side-upper", "Integrals", "Contour integral with side upper limit", "\\oint\\nolimits^{#?} #?"),
        ("oint-side-both", "Integrals", "Contour integral with side limits", "\\oint\\nolimits_{#?}^{#?} #?"),
        ("oint-stack-lower", "Integrals", "Contour integral with stacked lower limit", "\\oint\\limits_{#?} #?"),
        ("oint-stack-upper", "Integrals", "Contour integral with stacked upper limit", "\\oint\\limits^{#?} #?"),
        ("oint-stack-both", "Integrals", "Contour integral with stacked limits", "\\oint\\limits_{#?}^{#?} #?"),
        ("oiint-side-lower", "Integrals", "Surface integral with side lower limit", "\\oiint\\nolimits_{#?} #?"),
        ("oiint-side-upper", "Integrals", "Surface integral with side upper limit", "\\oiint\\nolimits^{#?} #?"),
        ("oiint-side-both", "Integrals", "Surface integral with side limits", "\\oiint\\nolimits_{#?}^{#?} #?"),
        ("oiint-stack-lower", "Integrals", "Surface integral with stacked lower limit", "\\oiint\\limits_{#?} #?"),
        ("oiint-stack-upper", "Integrals", "Surface integral with stacked upper limit", "\\oiint\\limits^{#?} #?"),
        ("oiint-stack-both", "Integrals", "Surface integral with stacked limits", "\\oiint\\limits_{#?}^{#?} #?"),
        ("oiiint-side-lower", "Integrals", "Volume integral with side lower limit", "\\oiiint\\nolimits_{#?} #?"),
        ("oiiint-side-upper", "Integrals", "Volume integral with side upper limit", "\\oiiint\\nolimits^{#?} #?"),
        ("oiiint-side-both", "Integrals", "Volume integral with side limits", "\\oiiint\\nolimits_{#?}^{#?} #?"),
        ("oiiint-stack-lower", "Integrals", "Volume integral with stacked lower limit", "\\oiiint\\limits_{#?} #?"),
        ("oiiint-stack-upper", "Integrals", "Volume integral with stacked upper limit", "\\oiiint\\limits^{#?} #?"),
        ("oiiint-stack-both", "Integrals", "Volume integral with stacked limits", "\\oiiint\\limits_{#?}^{#?} #?"),
        ("varointclockwise-side-lower", "Integrals", "Clockwise contour integral with side lower limit", "\\varointclockwise\\nolimits_{#?} #?"),
        ("varointclockwise-side-upper", "Integrals", "Clockwise contour integral with side upper limit", "\\varointclockwise\\nolimits^{#?} #?"),
        ("varointclockwise-side-both", "Integrals", "Clockwise contour integral with side limits", "\\varointclockwise\\nolimits_{#?}^{#?} #?"),
        ("varointclockwise-stack-lower", "Integrals", "Clockwise contour integral with stacked lower limit", "\\varointclockwise\\limits_{#?} #?"),
        ("varointclockwise-stack-upper", "Integrals", "Clockwise contour integral with stacked upper limit", "\\varointclockwise\\limits^{#?} #?"),
        ("varointclockwise-stack-both", "Integrals", "Clockwise contour integral with stacked limits", "\\varointclockwise\\limits_{#?}^{#?} #?"),
        ("ointctrclockwise-side-lower", "Integrals", "Anticlockwise contour integral with side lower limit", "\\ointctrclockwise\\nolimits_{#?} #?"),
        ("ointctrclockwise-side-upper", "Integrals", "Anticlockwise contour integral with side upper limit", "\\ointctrclockwise\\nolimits^{#?} #?"),
        ("ointctrclockwise-side-both", "Integrals", "Anticlockwise contour integral with side limits", "\\ointctrclockwise\\nolimits_{#?}^{#?} #?"),
        ("ointctrclockwise-stack-lower", "Integrals", "Anticlockwise contour integral with stacked lower limit", "\\ointctrclockwise\\limits_{#?} #?"),
        ("ointctrclockwise-stack-upper", "Integrals", "Anticlockwise contour integral with stacked upper limit", "\\ointctrclockwise\\limits^{#?} #?"),
        ("ointctrclockwise-stack-both", "Integrals", "Anticlockwise contour integral with stacked limits", "\\ointctrclockwise\\limits_{#?}^{#?} #?"),
        ("int-differential", "Integrals", "Indefinite integral (Ctrl+I)", "\\int #? \\, d#?"),
        ("definite-differential", "Integrals", "Definite integral (Ctrl+Shift+I)", "\\int\\limits_{#?}^{#?} #? \\, d#?"),
        ("prod-side-lower", "BigOperators", "Product with side lower limit", "\\prod\\nolimits_{#?} #?"),
        ("prod-side-upper", "BigOperators", "Product with side upper limit", "\\prod\\nolimits^{#?} #?"),
        ("prod-side-both", "BigOperators", "Product with side limits", "\\prod\\nolimits_{#?}^{#?} #?"),
        ("prod-stack-lower", "BigOperators", "Product with stacked lower limit", "\\prod\\limits_{#?} #?"),
        ("prod-stack-upper", "BigOperators", "Product with stacked upper limit", "\\prod\\limits^{#?} #?"),
        ("prod-stack-both", "BigOperators", "Product with stacked limits", "\\prod\\limits_{#?}^{#?} #?"),
        ("coprod-side-lower", "BigOperators", "Coproduct with side lower limit", "\\coprod\\nolimits_{#?} #?"),
        ("coprod-side-upper", "BigOperators", "Coproduct with side upper limit", "\\coprod\\nolimits^{#?} #?"),
        ("coprod-side-both", "BigOperators", "Coproduct with side limits", "\\coprod\\nolimits_{#?}^{#?} #?"),
        ("coprod-stack-lower", "BigOperators", "Coproduct with stacked lower limit", "\\coprod\\limits_{#?} #?"),
        ("coprod-stack-upper", "BigOperators", "Coproduct with stacked upper limit", "\\coprod\\limits^{#?} #?"),
        ("coprod-stack-both", "BigOperators", "Coproduct with stacked limits", "\\coprod\\limits_{#?}^{#?} #?"),
        ("bigcap-side-lower", "BigOperators", "Big intersection with side lower limit", "\\bigcap\\nolimits_{#?} #?"),
        ("bigcap-side-upper", "BigOperators", "Big intersection with side upper limit", "\\bigcap\\nolimits^{#?} #?"),
        ("bigcap-side-both", "BigOperators", "Big intersection with side limits", "\\bigcap\\nolimits_{#?}^{#?} #?"),
        ("bigcap-stack-lower", "BigOperators", "Big intersection with stacked lower limit", "\\bigcap\\limits_{#?} #?"),
        ("bigcap-stack-upper", "BigOperators", "Big intersection with stacked upper limit", "\\bigcap\\limits^{#?} #?"),
        ("bigcap-stack-both", "BigOperators", "Big intersection with stacked limits", "\\bigcap\\limits_{#?}^{#?} #?"),
        ("bigcup-side-lower", "BigOperators", "Big union with side lower limit", "\\bigcup\\nolimits_{#?} #?"),
        ("bigcup-side-upper", "BigOperators", "Big union with side upper limit", "\\bigcup\\nolimits^{#?} #?"),
        ("bigcup-side-both", "BigOperators", "Big union with side limits", "\\bigcup\\nolimits_{#?}^{#?} #?"),
        ("bigcup-stack-lower", "BigOperators", "Big union with stacked lower limit", "\\bigcup\\limits_{#?} #?"),
        ("bigcup-stack-upper", "BigOperators", "Big union with stacked upper limit", "\\bigcup\\limits^{#?} #?"),
        ("bigcup-stack-both", "BigOperators", "Big union with stacked limits", "\\bigcup\\limits_{#?}^{#?} #?"),
        ("bigwedge-side-lower", "BigOperators", "Big logical and with side lower limit", "\\bigwedge\\nolimits_{#?} #?"),
        ("bigwedge-side-upper", "BigOperators", "Big logical and with side upper limit", "\\bigwedge\\nolimits^{#?} #?"),
        ("bigwedge-side-both", "BigOperators", "Big logical and with side limits", "\\bigwedge\\nolimits_{#?}^{#?} #?"),
        ("bigwedge-stack-lower", "BigOperators", "Big logical and with stacked lower limit", "\\bigwedge\\limits_{#?} #?"),
        ("bigwedge-stack-upper", "BigOperators", "Big logical and with stacked upper limit", "\\bigwedge\\limits^{#?} #?"),
        ("bigwedge-stack-both", "BigOperators", "Big logical and with stacked limits", "\\bigwedge\\limits_{#?}^{#?} #?"),
        ("bigvee-side-lower", "BigOperators", "Big logical or with side lower limit", "\\bigvee\\nolimits_{#?} #?"),
        ("bigvee-side-upper", "BigOperators", "Big logical or with side upper limit", "\\bigvee\\nolimits^{#?} #?"),
        ("bigvee-side-both", "BigOperators", "Big logical or with side limits", "\\bigvee\\nolimits_{#?}^{#?} #?"),
        ("bigvee-stack-lower", "BigOperators", "Big logical or with stacked lower limit", "\\bigvee\\limits_{#?} #?"),
        ("bigvee-stack-upper", "BigOperators", "Big logical or with stacked upper limit", "\\bigvee\\limits^{#?} #?"),
        ("bigvee-stack-both", "BigOperators", "Big logical or with stacked limits", "\\bigvee\\limits_{#?}^{#?} #?"),
        ("bigotimes-side-lower", "BigOperators", "Big tensor product with side lower limit", "\\bigotimes\\nolimits_{#?} #?"),
        ("bigotimes-side-upper", "BigOperators", "Big tensor product with side upper limit", "\\bigotimes\\nolimits^{#?} #?"),
        ("bigotimes-side-both", "BigOperators", "Big tensor product with side limits", "\\bigotimes\\nolimits_{#?}^{#?} #?"),
        ("bigotimes-stack-lower", "BigOperators", "Big tensor product with stacked lower limit", "\\bigotimes\\limits_{#?} #?"),
        ("bigotimes-stack-upper", "BigOperators", "Big tensor product with stacked upper limit", "\\bigotimes\\limits^{#?} #?"),
        ("bigotimes-stack-both", "BigOperators", "Big tensor product with stacked limits", "\\bigotimes\\limits_{#?}^{#?} #?"),
        ("bigoplus-side-lower", "BigOperators", "Big direct sum with side lower limit", "\\bigoplus\\nolimits_{#?} #?"),
        ("bigoplus-side-upper", "BigOperators", "Big direct sum with side upper limit", "\\bigoplus\\nolimits^{#?} #?"),
        ("bigoplus-side-both", "BigOperators", "Big direct sum with side limits", "\\bigoplus\\nolimits_{#?}^{#?} #?"),
        ("bigoplus-stack-lower", "BigOperators", "Big direct sum with stacked lower limit", "\\bigoplus\\limits_{#?} #?"),
        ("bigoplus-stack-upper", "BigOperators", "Big direct sum with stacked upper limit", "\\bigoplus\\limits^{#?} #?"),
        ("bigoplus-stack-both", "BigOperators", "Big direct sum with stacked limits", "\\bigoplus\\limits_{#?}^{#?} #?"),
        ("bigodot-side-lower", "BigOperators", "Big dot product with side lower limit", "\\bigodot\\nolimits_{#?} #?"),
        ("bigodot-side-upper", "BigOperators", "Big dot product with side upper limit", "\\bigodot\\nolimits^{#?} #?"),
        ("bigodot-side-both", "BigOperators", "Big dot product with side limits", "\\bigodot\\nolimits_{#?}^{#?} #?"),
        ("bigodot-stack-lower", "BigOperators", "Big dot product with stacked lower limit", "\\bigodot\\limits_{#?} #?"),
        ("bigodot-stack-upper", "BigOperators", "Big dot product with stacked upper limit", "\\bigodot\\limits^{#?} #?"),
        ("bigodot-stack-both", "BigOperators", "Big dot product with stacked limits", "\\bigodot\\limits_{#?}^{#?} #?"),
        ("dot", "Embellishments", "Overdot", "\\dot{#?}"),
        ("ddot", "Embellishments", "Double overdot", "\\ddot{#?}"),
        ("dddot", "Embellishments", "Triple overdot", "\\dddot{#?}"),
        ("ddddot", "Embellishments", "Quadruple overdot", "\\ddddot{#?}"),
        ("hat", "Embellishments", "Hat", "\\hat{#?}"),
        ("widehat", "Embellishments", "Wide hat", "\\widehat{#?}"),
        ("tilde", "Embellishments", "Tilde", "\\tilde{#?}"),
        ("widetilde", "Embellishments", "Wide tilde", "\\widetilde{#?}"),
        ("check", "Embellishments", "Caron", "\\check{#?}"),
        ("breve", "Embellishments", "Breve", "\\breve{#?}"),
        ("mathring", "Embellishments", "Ring above", "\\mathring{#?}"),
        ("vec", "Embellishments", "Right vector (Ctrl+Alt+V)", "\\vec{#?}"),
        ("overleftarrow", "Embellishments", "Left vector", "\\overleftarrow{#?}"),
        ("overleftrightarrow", "Embellishments", "Left-right vector", "\\overleftrightarrow{#?}"),
        ("underdot", "Embellishments", "Underdot", "\\underdot{#?}"),
        ("underddot", "Embellishments", "Double underdot", "\\underddot{#?}"),
        ("underdddot", "Embellishments", "Triple underdot", "\\underdddot{#?}"),
        ("underddddot", "Embellishments", "Quadruple underdot", "\\underddddot{#?}"),
        ("underleftarrow", "Embellishments", "Left under-arrow", "\\underleftarrow{#?}"),
        ("underrightarrow", "Embellishments", "Right under-arrow", "\\underrightarrow{#?}"),
        ("underleftrightarrow", "Embellishments", "Left-right under-arrow", "\\underleftrightarrow{#?}"),
        ("underparen", "Embellishments", "Underarc", "\\underparen{#?}"),
        ("wideparen", "Embellishments", "Overarc", "\\wideparen{#?}"),
        ("cancel", "Embellishments", "Up-diagonal strike", "\\cancel{#?}"),
        ("bcancel", "Embellishments", "Down-diagonal strike", "\\bcancel{#?}"),
        ("xcancel", "Embellishments", "Cross strike", "\\xcancel{#?}"),
        ("strike-horizontal", "Embellishments", "Horizontal strike", "\\enclose{horizontalstrike}{#?}"),
        ("strike-vertical", "Embellishments", "Vertical strike", "\\enclose{verticalstrike}{#?}"),
        ("strike-horizontal-vertical", "Embellishments", "Crosshair strike", "\\enclose{horizontalstrike verticalstrike}{#?}"),
        ("prime", "Embellishments", "Prime", "{#?}'"),
        ("double-prime", "Embellishments", "Double prime", "{#?}''"),
        ("triple-prime", "Embellishments", "Triple prime", "{#?}'''"),
        ("quadruple-prime", "Embellishments", "Quadruple prime", "{#?}^{⁗}"),
        ("left-prime", "Embellishments", "Left prime", "\\prescript{\\prime}{}{#?}"),
        ("back-prime", "Embellishments", "Reversed prime", "{#?}^{\\backprime}"),
        ("underhat", "Embellishments", "Underhat", "\\underhat{#?}"),
        ("undertilde", "Embellishments", "Undertilde", "\\undertilde{#?}"),
        ("undercheck", "Embellishments", "Undercaron", "\\undercheck{#?}"),
        ("underbreve", "Embellishments", "Underbreve", "\\underbreve{#?}"),
        ("underring", "Embellishments", "Underring", "\\underring{#?}"),
        ("overleftharpoon", "Embellishments", "Left over-harpoon", "\\overleftharpoon{#?}"),
        ("overrightharpoon", "Embellishments", "Right over-harpoon", "\\overrightharpoon{#?}"),
        ("underleftharpoon", "Embellishments", "Left under-harpoon", "\\underleftharpoon{#?}"),
        ("underrightharpoon", "Embellishments", "Right under-harpoon", "\\underrightharpoon{#?}"),
        ("over-acute", "Embellishments", "Over acute accent", "\\overaccent{ˊ}{#?}"),
        ("under-acute", "Embellishments", "Under acute accent", "\\underaccent{ˊ}{#?}"),
        ("over-grave", "Embellishments", "Over grave accent", "\\overaccent{ˋ}{#?}"),
        ("under-grave", "Embellishments", "Under grave accent", "\\underaccent{ˋ}{#?}"),
        ("over-arrow-both", "Embellishments", "Over left-right arrow", "\\overaccent{↔}{#?}"),
        ("under-arrow-both", "Embellishments", "Under left-right arrow", "\\underaccent{↔}{#?}"),
        ("over-double-arrow-both", "Embellishments", "Over double left-right arrow", "\\overaccent{⇔}{#?}"),
        ("under-double-arrow-both", "Embellishments", "Under double left-right arrow", "\\underaccent{⇔}{#?}"),
        ("over-arrow-left", "Embellishments", "Over left arrow", "\\overaccent{←}{#?}"),
        ("under-arrow-left", "Embellishments", "Under left arrow", "\\underaccent{←}{#?}"),
        ("over-arrow-right", "Embellishments", "Over right arrow", "\\overaccent{→}{#?}"),
        ("under-arrow-right", "Embellishments", "Under right arrow", "\\underaccent{→}{#?}"),
        ("over-harpoon-left", "Embellishments", "Over left harpoon", "\\overaccent{↼}{#?}"),
        ("under-harpoon-left", "Embellishments", "Under left harpoon", "\\underaccent{↼}{#?}"),
        ("over-harpoon-right", "Embellishments", "Over right harpoon", "\\overaccent{⇀}{#?}"),
        ("under-harpoon-right", "Embellishments", "Under right harpoon", "\\underaccent{⇀}{#?}"),
        ("overline", "Bars", "Overbar", "\\overline{#?}"),
        ("underline", "Bars", "Underbar", "\\underline{#?}"),
        ("doubleoverline", "Bars", "Double overbar", "\\doubleoverline{#?}"),
        ("doubleunderline", "Bars", "Double underbar", "\\doubleunderline{#?}"),
        ("widehat", "Bars", "Wide hat", "\\widehat{#?}"),
        ("widetilde", "Bars", "Wide tilde", "\\widetilde{#?}"),
        ("wideparen", "Bars", "Overarc", "\\wideparen{#?}"),
        ("underparen", "Bars", "Underarc", "\\underparen{#?}"),
        ("overleftarrow", "Bars", "Left over-arrow", "\\overleftarrow{#?}"),
        ("overrightarrow", "Bars", "Right over-arrow", "\\overrightarrow{#?}"),
        ("overleftrightarrow", "Bars", "Left-right over-arrow", "\\overleftrightarrow{#?}"),
        ("underleftarrow", "Bars", "Left under-arrow", "\\underleftarrow{#?}"),
        ("underrightarrow", "Bars", "Right under-arrow", "\\underrightarrow{#?}"),
        ("underleftrightarrow", "Bars", "Left-right under-arrow", "\\underleftrightarrow{#?}"),
        ("overbrace-label", "Bars", "Labeled overbrace", "\\overbrace{#?}^{#?}"),
        ("underbrace-label", "Bars", "Labeled underbrace", "\\underbrace{#?}_{#?}"),
        ("overbracket-label", "Bars", "Labeled overbracket", "\\overbracket{#?}^{#?}"),
        ("underbracket-label", "Bars", "Labeled underbracket", "\\underbracket{#?}_{#?}"),
        ("overline-label", "Bars", "Labeled overbar", "\\overset{#?}{\\overline{#?}}"),
        ("underline-label", "Bars", "Labeled underbar", "\\underset{#?}{\\underline{#?}}"),
        ("doubleoverline-label", "Bars", "Labeled double overbar", "\\overset{#?}{\\doubleoverline{#?}}"),
        ("doubleunderline-label", "Bars", "Labeled double underbar", "\\underset{#?}{\\doubleunderline{#?}}"),
        ("strike-horizontalstrike", "Bars", "Horizontal strike", "\\enclose{horizontalstrike}{#?}"),
        ("strike-updiagonalstrike", "Bars", "Up-diagonal strike", "\\enclose{updiagonalstrike}{#?}"),
        ("strike-downdiagonalstrike", "Bars", "Down-diagonal strike", "\\enclose{downdiagonalstrike}{#?}"),
        ("strike-updiagonalstrike-downdiagonalstrike", "Bars", "Cross strike", "\\enclose{updiagonalstrike downdiagonalstrike}{#?}"),
        ("rightarrow-top", "LabeledArrows", "Right arrow with top label", "\\overset{#?}{\\rightarrow}"),
        ("rightarrow-bottom", "LabeledArrows", "Right arrow with bottom label", "\\underset{#?}{\\rightarrow}"),
        ("rightarrow-both", "LabeledArrows", "Right arrow with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\rightarrow}}"),
        ("leftarrow-top", "LabeledArrows", "Left arrow with top label", "\\overset{#?}{\\leftarrow}"),
        ("leftarrow-bottom", "LabeledArrows", "Left arrow with bottom label", "\\underset{#?}{\\leftarrow}"),
        ("leftarrow-both", "LabeledArrows", "Left arrow with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\leftarrow}}"),
        ("leftrightarrow-top", "LabeledArrows", "Left-right arrow with top label", "\\overset{#?}{\\leftrightarrow}"),
        ("leftrightarrow-bottom", "LabeledArrows", "Left-right arrow with bottom label", "\\underset{#?}{\\leftrightarrow}"),
        ("leftrightarrow-both", "LabeledArrows", "Left-right arrow with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\leftrightarrow}}"),
        ("Rightarrow-top", "LabeledArrows", "Double right arrow with top label", "\\overset{#?}{\\Rightarrow}"),
        ("Rightarrow-bottom", "LabeledArrows", "Double right arrow with bottom label", "\\underset{#?}{\\Rightarrow}"),
        ("Rightarrow-both", "LabeledArrows", "Double right arrow with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\Rightarrow}}"),
        ("Leftarrow-top", "LabeledArrows", "Double left arrow with top label", "\\overset{#?}{\\Leftarrow}"),
        ("Leftarrow-bottom", "LabeledArrows", "Double left arrow with bottom label", "\\underset{#?}{\\Leftarrow}"),
        ("Leftarrow-both", "LabeledArrows", "Double left arrow with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\Leftarrow}}"),
        ("Leftrightarrow-top", "LabeledArrows", "Double left-right arrow with top label", "\\overset{#?}{\\Leftrightarrow}"),
        ("Leftrightarrow-bottom", "LabeledArrows", "Double left-right arrow with bottom label", "\\underset{#?}{\\Leftrightarrow}"),
        ("Leftrightarrow-both", "LabeledArrows", "Double left-right arrow with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\Leftrightarrow}}"),
        ("rightharpoonup-top", "LabeledArrows", "Right harpoon with top label", "\\overset{#?}{\\rightharpoonup}"),
        ("rightharpoonup-bottom", "LabeledArrows", "Right harpoon with bottom label", "\\underset{#?}{\\rightharpoonup}"),
        ("rightharpoonup-both", "LabeledArrows", "Right harpoon with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\rightharpoonup}}"),
        ("leftharpoonup-top", "LabeledArrows", "Left harpoon with top label", "\\overset{#?}{\\leftharpoonup}"),
        ("leftharpoonup-bottom", "LabeledArrows", "Left harpoon with bottom label", "\\underset{#?}{\\leftharpoonup}"),
        ("leftharpoonup-both", "LabeledArrows", "Left harpoon with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\leftharpoonup}}"),
        ("rightleftharpoons-top", "LabeledArrows", "Right-left harpoons with top label", "\\overset{#?}{\\rightleftharpoons}"),
        ("rightleftharpoons-bottom", "LabeledArrows", "Right-left harpoons with bottom label", "\\underset{#?}{\\rightleftharpoons}"),
        ("rightleftharpoons-both", "LabeledArrows", "Right-left harpoons with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\rightleftharpoons}}"),
        ("leftrightharpoons-top", "LabeledArrows", "Left-right harpoons with top label", "\\overset{#?}{\\leftrightharpoons}"),
        ("leftrightharpoons-bottom", "LabeledArrows", "Left-right harpoons with bottom label", "\\underset{#?}{\\leftrightharpoons}"),
        ("leftrightharpoons-both", "LabeledArrows", "Left-right harpoons with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\leftrightharpoons}}"),
        ("mapsto-top", "LabeledArrows", "Maps-to arrow with top label", "\\overset{#?}{\\mapsto}"),
        ("mapsto-bottom", "LabeledArrows", "Maps-to arrow with bottom label", "\\underset{#?}{\\mapsto}"),
        ("mapsto-both", "LabeledArrows", "Maps-to arrow with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\mapsto}}"),
        ("rightleftarrows-top", "LabeledArrows", "Opposing arrows with top label", "\\overset{#?}{\\rightleftarrows}"),
        ("rightleftarrows-bottom", "LabeledArrows", "Opposing arrows with bottom label", "\\underset{#?}{\\rightleftarrows}"),
        ("rightleftarrows-both", "LabeledArrows", "Opposing arrows with top/bottom labels", "\\overset{#?}{\\underset{#?}{\\rightleftarrows}}"),
        ("plain-1x2", "Matrices", "1×2 matrix", "\\begin{matrix} #? & #? \\end{matrix}"),
        ("plain-2x1", "Matrices", "2×1 matrix", "\\begin{matrix} #? \\\\ #? \\end{matrix}"),
        ("plain-1x3", "Matrices", "1×3 matrix", "\\begin{matrix} #? & #? & #? \\end{matrix}"),
        ("plain-3x1", "Matrices", "3×1 matrix", "\\begin{matrix} #? \\\\ #? \\\\ #? \\end{matrix}"),
        ("plain-2x2", "Matrices", "2×2 matrix", "\\begin{matrix} #? & #? \\\\ #? & #? \\end{matrix}"),
        ("plain-3x3", "Matrices", "3×3 matrix", "\\begin{matrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{matrix}"),
        ("plain-2x3", "Matrices", "2×3 matrix", "\\begin{matrix} #? & #? & #? \\\\ #? & #? & #? \\end{matrix}"),
        ("plain-3x2", "Matrices", "3×2 matrix", "\\begin{matrix} #? & #? \\\\ #? & #? \\\\ #? & #? \\end{matrix}"),
        ("plain-1x4", "Matrices", "1×4 matrix", "\\begin{matrix} #? & #? & #? & #? \\end{matrix}"),
        ("plain-4x1", "Matrices", "4×1 matrix", "\\begin{matrix} #? \\\\ #? \\\\ #? \\\\ #? \\end{matrix}"),
        ("plain-4x4", "Matrices", "4×4 matrix", "\\begin{matrix} #? & #? & #? & #? \\\\ #? & #? & #? & #? \\\\ #? & #? & #? & #? \\\\ #? & #? & #? & #? \\end{matrix}"),
        ("pmatrix2", "Matrices", "2×2 matrix (parentheses)", "\\begin{pmatrix} #? & #? \\\\ #? & #? \\end{pmatrix}"),
        ("pmatrix3", "Matrices", "3×3 matrix (parentheses)", "\\begin{pmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{pmatrix}"),
        ("bmatrix2", "Matrices", "2×2 matrix (brackets)", "\\begin{bmatrix} #? & #? \\\\ #? & #? \\end{bmatrix}"),
        ("bmatrix3", "Matrices", "3×3 matrix (brackets)", "\\begin{bmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{bmatrix}"),
        ("Bmatrix2", "Matrices", "2×2 matrix (braces)", "\\begin{Bmatrix} #? & #? \\\\ #? & #? \\end{Bmatrix}"),
        ("Bmatrix3", "Matrices", "3×3 matrix (braces)", "\\begin{Bmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{Bmatrix}"),
        ("vmatrix2", "Matrices", "2×2 determinant", "\\begin{vmatrix} #? & #? \\\\ #? & #? \\end{vmatrix}"),
        ("vmatrix3", "Matrices", "3×3 determinant", "\\begin{vmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{vmatrix}"),
        ("Vmatrix2", "Matrices", "2×2 double-bar matrix", "\\begin{Vmatrix} #? & #? \\\\ #? & #? \\end{Vmatrix}"),
        ("Vmatrix3", "Matrices", "3×3 double-bar matrix", "\\begin{Vmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{Vmatrix}"),
        ("dots-row", "Matrices", "Row with ellipsis", "\\begin{matrix} #? & \\cdots & #? \\end{matrix}"),
        ("dots-column", "Matrices", "Column with ellipsis", "\\begin{matrix} #? \\\\ \\vdots \\\\ #? \\end{matrix}"),
        ("dots-matrix", "Matrices", "Matrix with ellipses", "\\begin{matrix} #? & \\cdots & #? \\\\ \\vdots & \\ddots & \\vdots \\\\ #? & \\cdots & #? \\end{matrix}"),
        ("aligned", "Matrices", "Aligned equations", "\\begin{aligned} #? &= #? \\\\ #? &= #? \\end{aligned}"),
        ("gathered", "Matrices", "Centered equations", "\\begin{gathered} #? \\\\ #? \\end{gathered}"),
        ("top-left", "Boxes", "Top-left corner box", "\\enclose{top left}{#?}"),
        ("top-right", "Boxes", "Top-right corner box", "\\enclose{top right}{#?}"),
        ("bottom-left", "Boxes", "Bottom-left corner box", "\\enclose{bottom left}{#?}"),
        ("bottom-right", "Boxes", "Bottom-right corner box", "\\enclose{bottom right}{#?}"),
        ("box", "Boxes", "Full box", "\\enclose{box}{#?}"),
        ("roundedbox", "Boxes", "Rounded box", "\\enclose{roundedbox}{#?}"),
        ("top-bottom-left", "Boxes", "Open-right box", "\\enclose{top bottom left}{#?}"),
        ("top-bottom-right", "Boxes", "Open-left box", "\\enclose{top bottom right}{#?}"),
        ("top-left-right", "Boxes", "Open-bottom box", "\\enclose{top left right}{#?}"),
        ("bottom-left-right", "Boxes", "Open-top box", "\\enclose{bottom left right}{#?}"),
        ("space-0.083333", "Spaces", "1-point space", "\\hspace{0.083333em}"),
        ("space-0.166667", "Spaces", "Thin space", "\\hspace{0.166667em}"),
        ("space-0.222222", "Spaces", "Medium space", "\\hspace{0.222222em}"),
        ("space-0.277778", "Spaces", "Thick space", "\\hspace{0.277778em}"),
        ("space-1", "Spaces", "1-em space", "\\hspace{1em}"),
        ("space-2", "Spaces", "2-em space", "\\hspace{2em}"),
    ],
    commands: [
        ("frac", "Fraction", "\\frac{#?}{#?}"),
        ("dfrac", "Display fraction", "\\dfrac{#?}{#?}"),
        ("tfrac", "Small fraction", "\\tfrac{#?}{#?}"),
        ("sfrac", "Bevelled fraction", "\\sfrac{#?}{#?}"),
        ("atopfrac", "Stacked without bar", "\\atopfrac{#?}{#?}"),
        ("sqrt", "Square root", "\\sqrt{#?}"),
        ("binom", "Binomial", "\\binom{#?}{#?}"),
        ("prescript", "Prescript", "\\prescript{#?}{#?}{#?}"),
        ("overset", "Overscript label", "\\overset{#?}{#?}"),
        ("underset", "Underscript label", "\\underset{#?}{#?}"),
        ("enclose", "Box enclosure", "\\enclose{box}{#?}"),
        ("boxed", "Full box", "\\boxed{#?}"),
        ("mathbb", "Blackboard bold", "\\mathbb{#?}"),
        ("mathfrak", "Fraktur", "\\mathfrak{#?}"),
        ("mathcal", "Calligraphic", "\\mathcal{#?}"),
        ("hspace", "Horizontal space", "\\hspace{1em}"),
        ("overbrace", "Overbrace", "\\overbrace{#?}"),
        ("underbrace", "Underbrace", "\\underbrace{#?}"),
        ("overbracket", "Overbracket", "\\overbracket{#?}"),
        ("underbracket", "Underbracket", "\\underbracket{#?}"),
        ("wideparen", "Overarc", "\\wideparen{#?}"),
        ("underparen", "Underarc", "\\underparen{#?}"),
        ("tsfrac", "Small inline fraction", "\\tsfrac{#?}{#?}"),
        ("dot", "Overdot", "\\dot{#?}"),
        ("ddot", "Double overdot", "\\ddot{#?}"),
        ("dddot", "Triple overdot", "\\dddot{#?}"),
        ("ddddot", "Quadruple overdot", "\\ddddot{#?}"),
        ("hat", "Hat", "\\hat{#?}"),
        ("widehat", "Wide hat", "\\widehat{#?}"),
        ("tilde", "Tilde", "\\tilde{#?}"),
        ("widetilde", "Wide tilde", "\\widetilde{#?}"),
        ("check", "Caron", "\\check{#?}"),
        ("breve", "Breve", "\\breve{#?}"),
        ("mathring", "Ring above", "\\mathring{#?}"),
        ("vec", "Right vector (Ctrl+Alt+V)", "\\vec{#?}"),
        ("overleftarrow", "Left vector", "\\overleftarrow{#?}"),
        ("overleftrightarrow", "Left-right vector", "\\overleftrightarrow{#?}"),
        ("underdot", "Underdot", "\\underdot{#?}"),
        ("underddot", "Double underdot", "\\underddot{#?}"),
        ("underdddot", "Triple underdot", "\\underdddot{#?}"),
        ("underddddot", "Quadruple underdot", "\\underddddot{#?}"),
        ("underleftarrow", "Left under-arrow", "\\underleftarrow{#?}"),
        ("underrightarrow", "Right under-arrow", "\\underrightarrow{#?}"),
        ("underleftrightarrow", "Left-right under-arrow", "\\underleftrightarrow{#?}"),
        ("cancel", "Up-diagonal strike", "\\cancel{#?}"),
        ("bcancel", "Down-diagonal strike", "\\bcancel{#?}"),
        ("xcancel", "Cross strike", "\\xcancel{#?}"),
        ("underhat", "Underhat", "\\underhat{#?}"),
        ("undertilde", "Undertilde", "\\undertilde{#?}"),
        ("undercheck", "Undercaron", "\\undercheck{#?}"),
        ("underbreve", "Underbreve", "\\underbreve{#?}"),
        ("underring", "Underring", "\\underring{#?}"),
        ("overleftharpoon", "Left over-harpoon", "\\overleftharpoon{#?}"),
        ("overrightharpoon", "Right over-harpoon", "\\overrightharpoon{#?}"),
        ("underleftharpoon", "Left under-harpoon", "\\underleftharpoon{#?}"),
        ("underrightharpoon", "Right under-harpoon", "\\underrightharpoon{#?}"),
        ("overaccent", "Over acute accent", "\\overaccent{ˊ}{#?}"),
        ("underaccent", "Under acute accent", "\\underaccent{ˊ}{#?}"),
        ("overline", "Overbar", "\\overline{#?}"),
        ("underline", "Underbar", "\\underline{#?}"),
        ("doubleoverline", "Double overbar", "\\doubleoverline{#?}"),
        ("doubleunderline", "Double underbar", "\\doubleunderline{#?}"),
        ("overrightarrow", "Right over-arrow", "\\overrightarrow{#?}"),
        ("lim", "Limit", "\\lim"),
        ("sin", "Sin", "\\sin"),
        ("cos", "Cos", "\\cos"),
        ("tan", "Tan", "\\tan"),
        ("cot", "Cotangent", "\\cot"),
        ("sec", "Secant", "\\sec"),
        ("csc", "Cosecant", "\\csc"),
        ("arcsin", "Arcsin", "\\arcsin"),
        ("arccos", "Arccos", "\\arccos"),
        ("arctan", "Arctan", "\\arctan"),
        ("sinh", "Sinh", "\\sinh"),
        ("cosh", "Cosh", "\\cosh"),
        ("tanh", "Tanh", "\\tanh"),
        ("log", "Logarithm", "\\log"),
        ("ln", "Natural logarithm", "\\ln"),
        ("exp", "Exponential", "\\exp"),
        ("min", "Minimum", "\\min"),
        ("max", "Maximum", "\\max"),
        ("inf", "Infimum", "\\inf"),
        ("sup", "Supremum", "\\sup"),
        ("det", "Determinant", "\\det"),
        ("to", "Right arrow", "\\rightarrow"),
        ("iff", "If and only if", "\\Leftrightarrow"),
        ("implies", "Implies", "\\Rightarrow"),
        ("leq", "Less than or equal to", "\\le"),
        ("geq", "Greater than or equal to", "\\ge"),
        ("ne", "Not equal to", "\\neq"),
        ("dots", "Ellipsis", "\\ldots"),
        ("bar", "Overbar", "\\overline{#?}"),
        ("pmatrix", "Parenthesized matrix", "\\begin{pmatrix} #? & #? \\\\ #? & #? \\end{pmatrix}"),
        ("bmatrix", "Bracketed matrix", "\\begin{bmatrix} #? & #? \\\\ #? & #? \\end{bmatrix}"),
        ("cases", "Piecewise cases", "\\begin{cases} #? & #? \\\\ #? & #? \\end{cases}"),
    ]
}

/// A control word's exact native symbol. Unknown words are left to the parser.
pub fn symbol(name: &str) -> Option<MathNode> {
    SYMBOLS
        .iter()
        .find(|s| s.name == name)
        .map(|s| MathNode::leaf(s.tag, s.glyph))
}

/// Parse a catalog recipe, replacing only explicit insertion markers with empty slots.
pub fn template(source: &str) -> Result<MathNode, String> {
    let mut source = source.to_string();
    if source.starts_with('^') || source.starts_with('_') {
        source = format!("{{#?}}{source}");
    }
    // Retain the legacy shortcut recipe used by existing sessions and tests.
    if source == "{}^{#?}_{#?}#?" {
        source = r"\prescript{#?}{#?}{#?}".into();
    }
    let source = source.replace("#?", r"\text{EQNTIDEINSERTIONSLOT}");
    let mut node = crate::service::parse_latex_to_mathml(&source).map_err(|e| e.to_string())?;
    fn placeholders(n: &mut MathNode) {
        if n.tag == "mtext" && n.text == "EQNTIDEINSERTIONSLOT" {
            *n = MathNode::new("mrow");
        }
        for child in &mut n.children {
            placeholders(child);
        }
    }
    placeholders(&mut node);
    if node.children.len() == 1 {
        Ok(node.children.remove(0))
    } else {
        node.tag = "mrow".into();
        Ok(node)
    }
}

/// Render the real insertion tree, decorating empty slots for the toolbar only.
pub fn preview_template(source: &str) -> Result<MathNode, String> {
    fn fill(node: &mut MathNode) {
        if node.children.is_empty()
            && node.text.is_empty()
            && matches!(node.tag.as_str(), "math" | "mrow" | "mtd")
        {
            node.children.push(MathNode::leaf("mi", "□"));
        } else {
            for child in &mut node.children {
                fill(child);
            }
        }
    }
    let mut node = template(source)?;
    fill(&mut node);
    if node.tag == "mspace" {
        // A spacing preset needs a visible indication of its width, not an empty icon.
        node = MathNode::with_children(
            "mrow",
            vec![MathNode::leaf("mo", "|"), node, MathNode::leaf("mo", "|")],
        );
    }
    Ok(MathNode::with_children("math", vec![node]))
}

/// Decorations and scripts attach to the preceding expression when nothing is selected.
pub fn attaches_to_previous(source: &str) -> bool {
    source.starts_with('^')
        || source.starts_with('_')
        || source.starts_with(r"\prescript")
        || ENTRIES.iter().any(|e| {
            e.source == source && matches!(e.group, "Scripts" | "Embellishments" | "Bars" | "Boxes")
        })
        || [
            r"\bar{",
            r"\boxed{",
            r"\enclose{",
            r"\overaccent{",
            r"\underaccent{",
        ]
        .iter()
        .any(|prefix| source.starts_with(prefix))
}

/// Construct an editable matrix. Inputs are bounded to the desktop dialog's limits.
/// `fence` is `none`, `parentheses`, `brackets`, `braces`, `bars`, or `double-bars`.
pub fn matrix(rows: usize, columns: usize, alignment: &str, fence: &str) -> MathNode {
    let rows = rows.clamp(1, 20);
    let columns = columns.clamp(1, 20);
    let mut table = MathNode::with_children(
        "mtable",
        (0..rows)
            .map(|_| {
                MathNode::with_children("mtr", (0..columns).map(|_| MathNode::new("mtd")).collect())
            })
            .collect(),
    );
    table.set_attr(
        "columnalign",
        match alignment {
            "left" => "left",
            "right" => "right",
            _ => "center",
        },
    );
    let (open, close) = match fence {
        "parentheses" => ("(", ")"),
        "brackets" => ("[", "]"),
        "braces" => ("{", "}"),
        "bars" => ("|", "|"),
        "double-bars" => ("‖", "‖"),
        _ => return table,
    };
    let mut left = MathNode::leaf("mo", open);
    left.set_attr("stretchy", "true");
    let mut right = MathNode::leaf("mo", close);
    right.set_attr("stretchy", "true");
    MathNode::with_children("mrow", vec![left, table, right])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fill_slots(n: &mut MathNode) {
        if matches!(n.tag.as_str(), "mrow" | "mtd") && n.children.is_empty() && n.text.is_empty() {
            n.children.push(MathNode::leaf("mi", "x"));
        }
        for child in &mut n.children {
            fill_slots(child);
        }
    }

    fn find_node<'a>(node: &'a MathNode, pred: &dyn Fn(&MathNode) -> bool) -> Option<&'a MathNode> {
        if pred(node) {
            return Some(node);
        }
        for child in &node.children {
            if let Some(found) = find_node(child, pred) {
                return Some(found);
            }
        }
        None
    }

    fn roundtrip_mtef(math: &MathNode) -> MathNode {
        let rec = crate::mathml_to_mtef_bytes(math).expect("mtef encode");
        crate::mtef::mtef_to_mathml(&crate::mtef::build_mtef(&rec))
            .expect("mtef decode should return Some")
    }

    fn roundtrip_omml(math: &MathNode) -> MathNode {
        let omml = crate::omml::mathml_to_omml(math).expect("omml encode");
        crate::omml::omml_to_mathml(&omml).expect("omml decode")
    }

    #[test]
    fn catalog_has_unique_ids_and_twenty_nonempty_groups() {
        let ids: std::collections::HashSet<_> = ENTRIES.iter().map(|e| e.id).collect();
        assert_eq!(ids.len(), ENTRIES.len());
        assert_eq!(GROUPS.len(), 20);
        for (group, _) in GROUPS {
            assert!(ENTRIES.iter().any(|e| e.group == *group), "{group}");
        }
        for e in ENTRIES {
            assert!(GROUPS.iter().any(|(g, _)| *g == e.group), "{}", e.id);
        }
    }

    #[test]
    fn every_catalog_recipe_is_insertable() {
        let errors: Vec<_> = ENTRIES
            .iter()
            .filter_map(|e| {
                template(e.source)
                    .err()
                    .map(|err| format!("{}: {}: {err}", e.id, e.source))
            })
            .collect();
        assert!(errors.is_empty(), "{}", errors.join("\n"));
    }

    #[test]
    fn literal_square_is_not_an_insertion_slot() {
        assert_eq!(template(r"\square").unwrap().text, "□");
        assert!(template(r"\frac{#?}{#?}")
            .unwrap()
            .children
            .iter()
            .all(|n| n.tag == "mrow" && n.children.is_empty()));
    }

    #[test]
    fn custom_matrix_bounds_alignment_and_fences() {
        let node = matrix(0, 30, "right", "brackets");
        assert_eq!(node.children[0].text, "[");
        let table = &node.children[1];
        assert_eq!(table.children.len(), 1);
        assert_eq!(table.children[0].children.len(), 20);
        assert_eq!(table.attr("columnalign"), Some("right"));
        let table = matrix(2, 3, "center", "none");
        assert_eq!(table.tag, "mtable");
        assert_eq!(table.children.len(), 2);
        assert_eq!(table.children[0].children.len(), 3);
    }

    #[test]
    fn every_catalog_entry_roundtrips_through_latex_mtef_omml_and_render() {
        for ch in "⟦⟧⊄≙ƛ⤢⤡↵∍◊∖".chars() {
            assert!(
                crate::render::font::fonts().has_glyph(ch),
                "missing font glyph for {ch} (U+{:04X})",
                ch as u32
            );
        }
        for e in ENTRIES {
            let mut node = template(e.source).expect("parse template");
            fill_slots(&mut node);
            let math = MathNode::with_children("math", vec![node]);

            let latex = math.to_latex();
            crate::service::parse_latex_to_mathml(&latex)
                .unwrap_or_else(|err| panic!("{}: latex roundtrip `{latex}` failed: {err}", e.id));

            let mtef_back = roundtrip_mtef(&math);
            assert!(
                !mtef_back.children.is_empty(),
                "{}: mtef decode produced empty math",
                e.id
            );

            let omml_back = roundtrip_omml(&math);
            assert!(
                !omml_back.children.is_empty(),
                "{}: omml decode produced empty math",
                e.id
            );

            let preview = crate::render::render_interactive(&math, 12.0)
                .unwrap_or_else(|err| panic!("{}: render_interactive failed: {err}", e.id));
            assert!(!preview.preview.png.is_empty(), "{}: empty PNG", e.id);
        }
    }

    #[test]
    fn catalog_semantic_attributes_survive_roundtrips() {
        // 1. Integral / sum / big-operator limit placement (\limits vs \nolimits).
        for (stack_src, side_src, op_glyph) in [
            (r"\int\limits_{a}^{b} x", r"\int\nolimits_{a}^{b} x", "∫"),
            (r"\sum\limits_{a}^{b} x", r"\sum\nolimits_{a}^{b} x", "∑"),
            (r"\prod\limits_{a}^{b} x", r"\prod\nolimits_{a}^{b} x", "∏"),
        ] {
            let stack_math = crate::service::parse_latex_to_mathml(stack_src).unwrap();
            let side_math = crate::service::parse_latex_to_mathml(side_src).unwrap();
            assert!(
                find_node(&stack_math, &|n| n.tag == "munderover").is_some(),
                "{stack_src}"
            );
            assert!(
                find_node(&side_math, &|n| n.tag == "msubsup").is_some(),
                "{side_src}"
            );

            for decoded in [
                crate::service::parse_latex_to_mathml(&stack_math.to_latex()).unwrap(),
                roundtrip_mtef(&stack_math),
                roundtrip_omml(&stack_math),
            ] {
                assert!(
                    find_node(&decoded, &|n| {
                        n.tag == "munderover"
                            && n.attr("data-limloc") != Some("subSup")
                            && find_node(n, &|c| c.text.trim() == op_glyph).is_some()
                    })
                    .is_some(),
                    "stacked limits lost for {stack_src}: {}",
                    decoded.to_xml()
                );
            }
            for decoded in [
                crate::service::parse_latex_to_mathml(&side_math.to_latex()).unwrap(),
                roundtrip_mtef(&side_math),
                roundtrip_omml(&side_math),
            ] {
                assert!(
                    find_node(&decoded, &|n| {
                        (n.tag == "msubsup"
                            || (n.tag == "munderover" && n.attr("data-limloc") == Some("subSup")))
                            && find_node(n, &|c| c.text.trim() == op_glyph).is_some()
                    })
                    .is_some(),
                    "side limits lost for {side_src}: {}",
                    decoded.to_xml()
                );
            }
        }

        // 2. Fraction variants (\dfrac, \tfrac, \sfrac, \tsfrac, \atopfrac).
        for (src, expected_cmd, attr_k, attr_v, preserved_in_omml, is_mtef_pile) in [
            (
                r"\dfrac{a}{b}",
                r"\dfrac",
                "displaystyle",
                "true",
                false,
                false,
            ),
            (
                r"\tfrac{a}{b}",
                r"\tfrac",
                "displaystyle",
                "false",
                false,
                false,
            ),
            (r"\sfrac{a}{b}", r"\sfrac", "bevelled", "true", true, false),
            (
                r"\tsfrac{a}{b}",
                r"\tsfrac",
                "bevelled",
                "true",
                true,
                false,
            ),
            (
                r"\atopfrac{a}{b}",
                r"\atopfrac",
                "linethickness",
                "0",
                true,
                true,
            ),
        ] {
            let math = crate::service::parse_latex_to_mathml(src).unwrap();
            assert!(
                find_node(&math, &|n| n.tag == "mfrac"
                    && n.attr(attr_k) == Some(attr_v))
                .is_some(),
                "{src}: {}",
                math.to_xml()
            );
            let latex_out = math.to_latex();
            assert!(latex_out.contains(expected_cmd), "{src} -> {latex_out}");
            let re_latex = crate::service::parse_latex_to_mathml(&latex_out).unwrap();
            assert!(
                find_node(&re_latex, &|n| n.tag == "mfrac"
                    && n.attr(attr_k) == Some(attr_v))
                .is_some(),
                "latex roundtrip lost {attr_k}={attr_v} for {src}: {}",
                re_latex.to_xml()
            );
            let omml_back = roundtrip_omml(&math);
            assert!(
                find_node(&omml_back, &|n| n.tag == "mfrac").is_some(),
                "omml lost mfrac for {src}: {}",
                omml_back.to_xml()
            );
            if preserved_in_omml {
                assert!(
                    find_node(&omml_back, &|n| n.tag == "mfrac"
                        && n.attr(attr_k) == Some(attr_v))
                    .is_some(),
                    "omml lost {attr_k}={attr_v} for {src}: {}",
                    omml_back.to_xml()
                );
            }
            let mtef_back = roundtrip_mtef(&math);
            if is_mtef_pile {
                assert!(
                    find_node(&mtef_back, &|n| n.tag == "mtable" && n.children.len() == 2)
                        .is_some(),
                    "mtef lost 2-row pile for {src}: {}",
                    mtef_back.to_xml()
                );
            } else {
                assert!(
                    find_node(&mtef_back, &|n| n.tag == "mfrac").is_some(),
                    "mtef lost mfrac for {src}: {}",
                    mtef_back.to_xml()
                );
                if expected_cmd != r"\dfrac" {
                    assert!(
                        find_node(&mtef_back, &|n| n.tag == "mfrac"
                            && n.attr(attr_k) == Some(attr_v))
                        .is_some(),
                        "mtef lost {attr_k}={attr_v} for {src}: {}",
                        mtef_back.to_xml()
                    );
                }
            }
        }

        // 3. Prescripts (\prescript{a}{b}{c}).
        let pre = crate::service::parse_latex_to_mathml(r"\prescript{a}{b}{c}").unwrap();
        assert!(pre.to_latex().contains(r"\prescript"));
        for decoded in [roundtrip_mtef(&pre), roundtrip_omml(&pre)] {
            assert!(
                find_node(&decoded, &|n| n.tag == "mprescripts").is_some(),
                "mprescripts lost: {}",
                decoded.to_xml()
            );
        }

        // 4. Font styles (mathvariant).
        for (src, variant) in [
            (r"\mathbb{R}", "double-struck"),
            (r"\mathfrak{A}", "fraktur"),
            (r"\mathcal{A}", "script"),
            (r"\mathbf{x}", "bold"),
            (r"\boldsymbol{\alpha}", "bold-italic"),
            (r"\mathrm{d}", "normal"),
        ] {
            let math = crate::service::parse_latex_to_mathml(src).unwrap();
            assert!(
                find_node(&math, &|n| n.attr("mathvariant") == Some(variant)).is_some(),
                "{src}"
            );
            let re_latex = crate::service::parse_latex_to_mathml(&math.to_latex()).unwrap();
            assert!(
                find_node(&re_latex, &|n| n.attr("mathvariant") == Some(variant)).is_some(),
                "latex roundtrip lost {variant} for {src}: {}",
                math.to_latex()
            );
            let omml_back = roundtrip_omml(&math);
            assert!(
                find_node(&omml_back, &|n| n.attr("mathvariant") == Some(variant)).is_some(),
                "omml roundtrip lost {variant} for {src}: {}",
                omml_back.to_xml()
            );
            let mtef_back = roundtrip_mtef(&math);
            assert!(
                !mtef_back.children.is_empty(),
                "mtef roundtrip failed for {src}"
            );
        }

        // 5. Enclosures, strikes, double bars, accents.
        for (src, notation_token) in [
            (r"\boxed{x}", "box"),
            (r"\cancel{x}", "updiagonalstrike"),
            (r"\bcancel{x}", "downdiagonalstrike"),
            (r"\xcancel{x}", "updiagonalstrike"),
            (r"\enclose{top left}{x}", "top"),
            (r"\enclose{top bottom left}{x}", "bottom"),
            (r"\enclose{horizontalstrike}{x}", "horizontalstrike"),
        ] {
            let math = crate::service::parse_latex_to_mathml(src).unwrap();
            for decoded in [
                crate::service::parse_latex_to_mathml(&math.to_latex()).unwrap(),
                roundtrip_mtef(&math),
                roundtrip_omml(&math),
            ] {
                assert!(
                    find_node(&decoded, &|n| {
                        n.tag == "menclose"
                            && n.attr("notation")
                                .is_some_and(|v| v.split_whitespace().any(|t| t == notation_token))
                    })
                    .is_some(),
                    "menclose({notation_token}) lost for {src}: {}",
                    decoded.to_xml()
                );
            }
        }
        for src in [
            r"\doubleoverline{x}",
            r"\doubleunderline{x}",
            r"\overaccent{↔}{x}",
            r"\underaccent{↔}{x}",
        ] {
            let math = crate::service::parse_latex_to_mathml(src).unwrap();
            assert!(!math.to_latex().is_empty());
            let _ = roundtrip_mtef(&math);
            let _ = roundtrip_omml(&math);
        }

        // 6. Matrices (array alignment, Bmatrix, Vmatrix, 4x4).
        let arr =
            crate::service::parse_latex_to_mathml(r"\begin{array}{lcr} a & b & c \end{array}")
                .unwrap();
        assert_eq!(
            find_node(&arr, &|n| n.tag == "mtable").and_then(|t| t.attr("columnalign")),
            Some("left center right")
        );
        for decoded in [
            crate::service::parse_latex_to_mathml(&arr.to_latex()).unwrap(),
            roundtrip_omml(&arr),
        ] {
            assert_eq!(
                find_node(&decoded, &|n| n.tag == "mtable").and_then(|t| t.attr("columnalign")),
                Some("left center right"),
                "columnalign lost: {}",
                decoded.to_xml()
            );
        }
        for (src, delim) in [
            (r"\begin{Bmatrix} a & b \\ c & d \end{Bmatrix}", "{"),
            (r"\begin{Vmatrix} a & b \\ c & d \end{Vmatrix}", "‖"),
        ] {
            let math = crate::service::parse_latex_to_mathml(src).unwrap();
            assert!(find_node(&math, &|n| n.tag == "mo" && n.text.trim() == delim).is_some());
            assert!(find_node(&roundtrip_mtef(&math), &|n| n.tag == "mtable").is_some());
            assert!(find_node(&roundtrip_omml(&math), &|n| n.tag == "mtable").is_some());
        }
        let mut custom = matrix(4, 4, "right", "brackets");
        fill_slots(&mut custom);
        let custom_math = MathNode::with_children("math", vec![custom]);
        assert_eq!(
            find_node(&roundtrip_omml(&custom_math), &|n| n.tag == "mtable")
                .map(|t| t.children.len()),
            Some(4)
        );

        // 7. Labeled arrows.
        for (src, arrow_glyph) in [
            (r"\overset{a}{\rightarrow}", "→"),
            (r"\underset{b}{\leftarrow}", "←"),
            (r"\overset{a}{\underset{b}{\leftrightarrow}}", "↔"),
            (r"\overset{a}{\rightleftharpoons}", "⇌"),
        ] {
            let math = crate::service::parse_latex_to_mathml(src).unwrap();
            for decoded in [
                crate::service::parse_latex_to_mathml(&math.to_latex()).unwrap(),
                roundtrip_mtef(&math),
                roundtrip_omml(&math),
            ] {
                assert!(
                    find_node(&decoded, &|n| n.tag == "mo" && n.text.trim() == arrow_glyph)
                        .is_some(),
                    "arrow `{arrow_glyph}` lost for {src}: {}",
                    decoded.to_xml()
                );
            }
        }

        // 8. Explicit spaces.
        for width in [
            "0.083333em",
            "0.166667em",
            "0.222222em",
            "0.277778em",
            "1em",
            "2em",
        ] {
            let src = format!(r"x\hspace{{{width}}}y");
            let math = crate::service::parse_latex_to_mathml(&src).unwrap();
            assert!(
                find_node(&math, &|n| n.tag == "mspace"
                    && n.attr("width") == Some(width))
                .is_some(),
                "{src}"
            );
            assert!(find_node(&roundtrip_mtef(&math), &|n| n.tag == "mspace").is_some());
            assert!(find_node(&roundtrip_omml(&math), &|n| n.tag == "mspace").is_some());
        }

        // 9. Half-open, Dirac, and double-bracket fences.
        for (src, open_ch, close_ch) in [
            (r"\left[ x \right)", Some("["), Some(")")),
            (r"\left( x \right]", Some("("), Some("]")),
            (r"\left| x \right\rangle", Some("|"), Some("⟩")),
            (r"\left\langle x \right|", Some("⟨"), Some("|")),
            (r"\left[ x \right[", Some("["), Some("[")),
            (r"\left] x \right]", Some("]"), Some("]")),
            (r"\left] x \right[", Some("]"), Some("[")),
            (r"\left\llbracket x \right\rrbracket", Some("⟦"), Some("⟧")),
            (r"\left\llbracket x \right.", Some("⟦"), None),
            (r"\left. x \right\rrbracket", None, Some("⟧")),
        ] {
            let math = crate::service::parse_latex_to_mathml(src).unwrap();
            for decoded in [
                crate::service::parse_latex_to_mathml(&math.to_latex()).unwrap(),
                roundtrip_mtef(&math),
                roundtrip_omml(&math),
            ] {
                if let Some(open) = open_ch {
                    assert!(
                        find_node(&decoded, &|n| {
                            n.tag == "mo"
                                && (n.text.trim() == open
                                    || (open == "⟦" && n.text.trim() == "〚")
                                    || (open == "⟨" && n.text.trim() == "〈"))
                        })
                        .is_some(),
                        "open `{open}` lost for {src}: {}",
                        decoded.to_xml()
                    );
                }
                if let Some(close) = close_ch {
                    assert!(
                        find_node(&decoded, &|n| {
                            n.tag == "mo"
                                && (n.text.trim() == close
                                    || (close == "⟧" && n.text.trim() == "〛")
                                    || (close == "⟩" && n.text.trim() == "〉"))
                        })
                        .is_some(),
                        "close `{close}` lost for {src}: {}",
                        decoded.to_xml()
                    );
                }
            }
        }
    }
}
