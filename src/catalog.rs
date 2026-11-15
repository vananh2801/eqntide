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
    ("Fences", "Ngoặc"),
    ("Fractions", "Phân số & căn"),
    ("Scripts", "Chỉ số"),
    ("Integrals", "Tích phân"),
    ("Sums", "Tổng"),
    ("BigOperators", "Tích & phép toán lớn"),
    ("Matrices", "Ma trận"),
    ("Embellishments", "Dấu trên/dưới"),
    ("Bars", "Gạch trên/dưới"),
    ("LabeledArrows", "Mũi tên có nhãn"),
    ("Boxes", "Khung"),
    ("Relations", "Quan hệ"),
    ("Operators", "Toán tử"),
    ("Arrows", "Mũi tên"),
    ("Logic", "Logic"),
    ("Sets", "Tập hợp"),
    ("Symbols", "Ký hiệu khác"),
    ("GreekLower", "Hy Lạp thường"),
    ("GreekUpper", "Hy Lạp hoa"),
    ("Spaces", "Khoảng trắng & dấu chấm"),
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
        ("Relations", "le", "≤", "Nhỏ hơn hoặc bằng", "mo"),
        ("Relations", "ge", "≥", "Lớn hơn hoặc bằng", "mo"),
        ("Relations", "ll", "≪", "Rất nhỏ hơn", "mo"),
        ("Relations", "gg", "≫", "Rất lớn hơn", "mo"),
        ("Relations", "prec", "≺", "Đứng trước", "mo"),
        ("Relations", "succ", "≻", "Đứng sau", "mo"),
        ("Relations", "triangleleft", "◁", "Tam giác trái", "mo"),
        ("Relations", "triangleright", "▷", "Tam giác phải", "mo"),
        ("Relations", "sim", "∼", "Tương tự", "mo"),
        ("Relations", "approx", "≈", "Xấp xỉ", "mo"),
        ("Relations", "simeq", "≃", "Tương đương xấp xỉ", "mo"),
        ("Relations", "cong", "≅", "Đồng dạng", "mo"),
        ("Relations", "neq", "≠", "Khác", "mo"),
        ("Relations", "equiv", "≡", "Đồng nhất", "mo"),
        ("Relations", "doteq", "≐", "Bằng theo định nghĩa", "mo"),
        ("Relations", "triangleq", "≜", "Định nghĩa bằng", "mo"),
        ("Relations", "estimates", "≙", "Ước lượng bằng", "mo"),
        ("Relations", "fallingdotseq", "≒", "Xấp xỉ bằng", "mo"),
        ("Relations", "risingdotseq", "≓", "Bằng xấp xỉ", "mo"),
        ("Relations", "asymp", "≍", "Tương đương tiệm cận", "mo"),
        ("Relations", "propto", "∝", "Tỉ lệ thuận", "mo"),
        ("Relations", "nleq", "≰", "Không nhỏ hơn hoặc bằng", "mo"),
        ("Relations", "ngeq", "≱", "Không lớn hơn hoặc bằng", "mo"),
        ("Relations", "preceq", "≼", "Đứng trước hoặc bằng", "mo"),
        ("Relations", "succeq", "≽", "Đứng sau hoặc bằng", "mo"),
        ("Operators", "pm", "±", "Cộng trừ", "mo"),
        ("Operators", "mp", "∓", "Trừ cộng", "mo"),
        ("Operators", "times", "×", "Nhân", "mo"),
        ("Operators", "ast", "∗", "Dấu sao", "mo"),
        ("Operators", "div", "÷", "Chia", "mo"),
        ("Operators", "oplus", "⊕", "Cộng trong vòng tròn", "mo"),
        ("Operators", "otimes", "⊗", "Nhân trong vòng tròn", "mo"),
        ("Operators", "odot", "⊙", "Chấm trong vòng tròn", "mo"),
        ("Operators", "cdot", "⋅", "Nhân chấm", "mo"),
        ("Operators", "bullet", "•", "Chấm đậm", "mo"),
        ("Operators", "circ", "∘", "Hợp thành", "mo"),
        ("Operators", "smallcirc", "○", "Vòng tròn", "mo"),
        ("Operators", "diamond", "⋄", "Hình thoi", "mo"),
        ("Operators", "dagger", "†", "Dấu thập", "mo"),
        ("Operators", "ddagger", "‡", "Dấu thập kép", "mo"),
        ("Operators", "amalg", "⨿", "Hợp rời", "mo"),
        ("Operators", "langle", "⟨", "Ngoặc góc trái", "mo"),
        ("Operators", "rangle", "⟩", "Ngoặc góc phải", "mo"),
        ("Operators", "llbracket", "⟦", "Ngoặc vuông kép trái", "mo"),
        ("Operators", "rrbracket", "⟧", "Ngoặc vuông kép phải", "mo"),
        ("Operators", "Vert", "‖", "Vạch kép", "mo"),
        ("Operators", "parallel", "∥", "Song song", "mo"),
        ("Operators", "perp", "⊥", "Vuông góc", "mo"),
        ("Arrows", "leftrightarrow", "↔", "Hai chiều", "mo"),
        ("Arrows", "rightarrow", "→", "Sang phải", "mo"),
        ("Arrows", "leftarrow", "←", "Sang trái", "mo"),
        ("Arrows", "updownarrow", "↕", "Lên xuống", "mo"),
        ("Arrows", "uparrow", "↑", "Lên", "mo"),
        ("Arrows", "downarrow", "↓", "Xuống", "mo"),
        ("Arrows", "Leftrightarrow", "⇔", "Hai chiều kép", "mo"),
        ("Arrows", "Rightarrow", "⇒", "Suy ra", "mo"),
        ("Arrows", "Leftarrow", "⇐", "Suy ra trái", "mo"),
        ("Arrows", "Updownarrow", "⇕", "Lên xuống kép", "mo"),
        ("Arrows", "Uparrow", "⇑", "Lên kép", "mo"),
        ("Arrows", "Downarrow", "⇓", "Xuống kép", "mo"),
        ("Arrows", "nearrow", "↗", "Đông bắc", "mo"),
        ("Arrows", "searrow", "↘", "Đông nam", "mo"),
        ("Arrows", "swarrow", "↙", "Tây nam", "mo"),
        ("Arrows", "nwarrow", "↖", "Tây bắc", "mo"),
        ("Arrows", "neswarrow", "⤢", "Mũi tên chéo đông bắc–tây nam", "mo"),
        ("Arrows", "nwsearrow", "⤡", "Mũi tên chéo tây bắc–đông nam", "mo"),
        ("Arrows", "leftrightarrows", "⇆", "Hai mũi tên ngược chiều", "mo"),
        ("Arrows", "rightleftarrows", "⇄", "Hai mũi tên đổi chiều", "mo"),
        ("Arrows", "rightleftharpoons", "⇌", "Cân bằng thuận", "mo"),
        ("Arrows", "leftrightharpoons", "⇋", "Cân bằng nghịch", "mo"),
        ("Arrows", "rightharpoonup", "⇀", "Móc phải trên", "mo"),
        ("Arrows", "rightharpoondown", "⇁", "Móc phải dưới", "mo"),
        ("Arrows", "leftharpoonup", "↼", "Móc trái trên", "mo"),
        ("Arrows", "leftharpoondown", "↽", "Móc trái dưới", "mo"),
        ("Arrows", "mapsto", "↦", "Ánh xạ", "mo"),
        ("Arrows", "hookleftarrow", "↩", "Móc trái", "mo"),
        ("Arrows", "hookrightarrow", "↪", "Móc phải", "mo"),
        ("Arrows", "longrightarrow", "⟶", "Mũi tên phải dài", "mo"),
        ("Arrows", "longleftarrow", "⟵", "Mũi tên trái dài", "mo"),
        ("Arrows", "longleftrightarrow", "⟷", "Mũi tên hai chiều dài", "mo"),
        ("Arrows", "Longrightarrow", "⟹", "Mũi tên phải kép dài", "mo"),
        ("Arrows", "Longleftarrow", "⟸", "Mũi tên trái kép dài", "mo"),
        ("Arrows", "Longleftrightarrow", "⟺", "Mũi tên hai chiều kép dài", "mo"),
        ("Arrows", "curvearrowleft", "↶", "Vòng trái", "mo"),
        ("Arrows", "curvearrowright", "↷", "Vòng phải", "mo"),
        ("Arrows", "carriagereturn", "↵", "Mũi tên xuống rồi sang trái", "mo"),
        ("Logic", "therefore", "∴", "Do đó", "mo"),
        ("Logic", "because", "∵", "Bởi vì", "mo"),
        ("Logic", "exists", "∃", "Tồn tại", "mo"),
        ("Logic", "backepsilon", "∍", "Sao cho", "mo"),
        ("Logic", "nexists", "∄", "Không tồn tại", "mo"),
        ("Logic", "forall", "∀", "Với mọi", "mo"),
        ("Logic", "neg", "¬", "Phủ định", "mo"),
        ("Logic", "land", "∧", "Và", "mo"),
        ("Logic", "lor", "∨", "Hoặc", "mo"),
        ("Logic", "top", "⊤", "Đúng", "mo"),
        ("Logic", "bot", "⊥", "Sai", "mo"),
        ("Logic", "vdash", "⊢", "Suy diễn", "mo"),
        ("Logic", "dashv", "⊣", "Suy diễn ngược", "mo"),
        ("Logic", "models", "⊨", "Thoả mãn", "mo"),
        ("Sets", "in", "∈", "Thuộc", "mo"),
        ("Sets", "notin", "∉", "Không thuộc", "mo"),
        ("Sets", "ni", "∋", "Chứa phần tử", "mo"),
        ("Sets", "notni", "∌", "Không chứa phần tử", "mo"),
        ("Sets", "cup", "∪", "Hợp", "mo"),
        ("Sets", "cap", "∩", "Giao", "mo"),
        ("Sets", "subset", "⊂", "Tập con", "mo"),
        ("Sets", "supset", "⊃", "Tập cha", "mo"),
        ("Sets", "nsubset", "⊄", "Không là tập con", "mo"),
        ("Sets", "subseteq", "⊆", "Tập con hoặc bằng", "mo"),
        ("Sets", "supseteq", "⊇", "Tập cha hoặc bằng", "mo"),
        ("Sets", "nsubseteq", "⊈", "Không là tập con hoặc bằng", "mo"),
        ("Sets", "nsupseteq", "⊉", "Không là tập cha hoặc bằng", "mo"),
        ("Sets", "emptyset", "∅", "Tập rỗng", "mo"),
        ("Sets", "setminus", "∖", "Hiệu tập hợp", "mo"),
        ("Sets", "subsetneq", "⊊", "Tập con thực sự", "mo"),
        ("Sets", "supsetneq", "⊋", "Tập cha thực sự", "mo"),
        ("Symbols", "partial", "∂", "Đạo hàm riêng", "mo"),
        ("Symbols", "ell", "ℓ", "Chữ l viết tay", "mi"),
        ("Symbols", "wp", "℘", "Hàm Weierstrass", "mi"),
        ("Symbols", "Re", "ℜ", "Phần thực", "mi"),
        ("Symbols", "Im", "ℑ", "Phần ảo", "mi"),
        ("Symbols", "aleph", "ℵ", "Aleph", "mi"),
        ("Symbols", "infty", "∞", "Vô cực", "mo"),
        ("Symbols", "hbar", "ℏ", "Hằng số Planck rút gọn", "mi"),
        ("Symbols", "lambdabar", "ƛ", "Lambda gạch ngang", "mi"),
        ("Symbols", "imath", "ı", "Chữ i không chấm", "mi"),
        ("Symbols", "jmath", "ȷ", "Chữ j không chấm", "mi"),
        ("Symbols", "nabla", "∇", "Nabla", "mo"),
        ("Symbols", "triangle", "△", "Tam giác", "mo"),
        ("Symbols", "mho", "℧", "Mho", "mo"),
        ("Symbols", "angle", "∠", "Góc", "mo"),
        ("Symbols", "measuredangle", "∡", "Góc đo", "mo"),
        ("Symbols", "sphericalangle", "∢", "Góc cầu", "mo"),
        ("Symbols", "square", "□", "Hình vuông", "mo"),
        ("Symbols", "Box", "□", "Ô vuông", "mo"),
        ("Symbols", "rectangle", "▭", "Hình chữ nhật", "mo"),
        ("Symbols", "parallelogram", "▱", "Hình bình hành", "mo"),
        ("Symbols", "bigcirc", "◯", "Đường tròn", "mo"),
        ("Symbols", "lozenge", "◊", "Hình thoi rỗng", "mo"),
        ("Symbols", "degree", "°", "Độ", "mo"),
        ("Symbols", "prime", "′", "Phẩy trên", "mo"),
        ("Symbols", "backprime", "‵", "Phẩy trên ngược", "mo"),
        ("Symbols", "clubsuit", "♣", "Chuồn", "mo"),
        ("Symbols", "heartsuit", "♡", "Cơ", "mo"),
        ("Symbols", "diamondsuit", "♢", "Rô", "mo"),
        ("Symbols", "spadesuit", "♠", "Bích", "mo"),
        ("GreekLower", "alpha", "α", "Hy Lạp alpha", "mi"),
        ("GreekLower", "beta", "β", "Hy Lạp beta", "mi"),
        ("GreekLower", "gamma", "γ", "Hy Lạp gamma", "mi"),
        ("GreekLower", "delta", "δ", "Hy Lạp delta", "mi"),
        ("GreekLower", "epsilon", "ϵ", "Hy Lạp epsilon", "mi"),
        ("GreekLower", "phi", "ϕ", "Hy Lạp phi", "mi"),
        ("GreekLower", "varphi", "φ", "Hy Lạp varphi", "mi"),
        ("GreekLower", "vartheta", "ϑ", "Hy Lạp vartheta", "mi"),
        ("GreekLower", "eta", "η", "Hy Lạp eta", "mi"),
        ("GreekLower", "iota", "ι", "Hy Lạp iota", "mi"),
        ("GreekLower", "kappa", "κ", "Hy Lạp kappa", "mi"),
        ("GreekLower", "lambda", "λ", "Hy Lạp lambda", "mi"),
        ("GreekLower", "mu", "μ", "Hy Lạp mu", "mi"),
        ("GreekLower", "nu", "ν", "Hy Lạp nu", "mi"),
        ("GreekLower", "omicron", "ο", "Hy Lạp omicron", "mi"),
        ("GreekLower", "pi", "π", "Hy Lạp pi", "mi"),
        ("GreekLower", "varpi", "ϖ", "Hy Lạp varpi", "mi"),
        ("GreekLower", "theta", "θ", "Hy Lạp theta", "mi"),
        ("GreekLower", "rho", "ρ", "Hy Lạp rho", "mi"),
        ("GreekLower", "varrho", "ϱ", "Hy Lạp varrho", "mi"),
        ("GreekLower", "sigma", "σ", "Hy Lạp sigma", "mi"),
        ("GreekLower", "varsigma", "ς", "Hy Lạp varsigma", "mi"),
        ("GreekLower", "tau", "τ", "Hy Lạp tau", "mi"),
        ("GreekLower", "upsilon", "υ", "Hy Lạp upsilon", "mi"),
        ("GreekLower", "omega", "ω", "Hy Lạp omega", "mi"),
        ("GreekLower", "xi", "ξ", "Hy Lạp xi", "mi"),
        ("GreekLower", "psi", "ψ", "Hy Lạp psi", "mi"),
        ("GreekLower", "zeta", "ζ", "Hy Lạp zeta", "mi"),
        ("GreekLower", "chi", "χ", "Hy Lạp chi", "mi"),
        ("GreekLower", "varepsilon", "ε", "Hy Lạp varepsilon", "mi"),
        ("GreekLower", "varkappa", "ϰ", "Hy Lạp varkappa", "mi"),
        ("GreekLower", "digamma", "ϝ", "Hy Lạp digamma", "mi"),
        ("GreekUpper", "Alpha", "Α", "Hy Lạp Alpha", "mi"),
        ("GreekUpper", "Beta", "Β", "Hy Lạp Beta", "mi"),
        ("GreekUpper", "Chi", "Χ", "Hy Lạp Chi", "mi"),
        ("GreekUpper", "Delta", "Δ", "Hy Lạp Delta", "mi"),
        ("GreekUpper", "Epsilon", "Ε", "Hy Lạp Epsilon", "mi"),
        ("GreekUpper", "Phi", "Φ", "Hy Lạp Phi", "mi"),
        ("GreekUpper", "Gamma", "Γ", "Hy Lạp Gamma", "mi"),
        ("GreekUpper", "Eta", "Η", "Hy Lạp Eta", "mi"),
        ("GreekUpper", "Iota", "Ι", "Hy Lạp Iota", "mi"),
        ("GreekUpper", "Kappa", "Κ", "Hy Lạp Kappa", "mi"),
        ("GreekUpper", "Lambda", "Λ", "Hy Lạp Lambda", "mi"),
        ("GreekUpper", "Mu", "Μ", "Hy Lạp Mu", "mi"),
        ("GreekUpper", "Nu", "Ν", "Hy Lạp Nu", "mi"),
        ("GreekUpper", "Omicron", "Ο", "Hy Lạp Omicron", "mi"),
        ("GreekUpper", "Pi", "Π", "Hy Lạp Pi", "mi"),
        ("GreekUpper", "Theta", "Θ", "Hy Lạp Theta", "mi"),
        ("GreekUpper", "Rho", "Ρ", "Hy Lạp Rho", "mi"),
        ("GreekUpper", "Sigma", "Σ", "Hy Lạp Sigma", "mi"),
        ("GreekUpper", "Tau", "Τ", "Hy Lạp Tau", "mi"),
        ("GreekUpper", "Upsilon", "Υ", "Hy Lạp Upsilon", "mi"),
        ("GreekUpper", "Omega", "Ω", "Hy Lạp Omega", "mi"),
        ("GreekUpper", "Xi", "Ξ", "Hy Lạp Xi", "mi"),
        ("GreekUpper", "Psi", "Ψ", "Hy Lạp Psi", "mi"),
        ("GreekUpper", "Zeta", "Ζ", "Hy Lạp Zeta", "mi"),
        ("GreekUpper", "Digamma", "Ϝ", "Hy Lạp Digamma", "mi"),
        ("GreekUpper", "varTheta", "ϴ", "Hy Lạp varTheta", "mi"),
        ("GreekUpper", "varUpsilon", "ϒ", "Hy Lạp varUpsilon", "mi"),
        ("Sums", "sum", "∑", "Tổng", "mo"),
        ("Integrals", "int", "∫", "Tích phân", "mo"),
        ("Integrals", "iint", "∬", "Tích phân kép", "mo"),
        ("Integrals", "iiint", "∭", "Tích phân ba", "mo"),
        ("Integrals", "oint", "∮", "Tích phân vòng", "mo"),
        ("Integrals", "oiint", "∯", "Tích phân mặt kín", "mo"),
        ("Integrals", "oiiint", "∰", "Tích phân khối kín", "mo"),
        ("Integrals", "varointclockwise", "∲", "Tích phân vòng thuận chiều", "mo"),
        ("Integrals", "ointctrclockwise", "∳", "Tích phân vòng ngược chiều", "mo"),
        ("BigOperators", "prod", "∏", "Tích", "mo"),
        ("BigOperators", "coprod", "∐", "Đồng tích", "mo"),
        ("BigOperators", "bigcap", "⋂", "Giao lớn", "mo"),
        ("BigOperators", "bigcup", "⋃", "Hợp lớn", "mo"),
        ("BigOperators", "bigwedge", "⋀", "Hội lớn", "mo"),
        ("BigOperators", "bigvee", "⋁", "Tuyển lớn", "mo"),
        ("BigOperators", "bigotimes", "⨂", "Tích tensor lớn", "mo"),
        ("BigOperators", "bigoplus", "⨁", "Tổng trực tiếp lớn", "mo"),
        ("BigOperators", "bigodot", "⨀", "Tích chấm lớn", "mo"),
        ("Spaces", "ldots", "…", "Ba chấm ngang thấp", "mo"),
        ("Spaces", "cdots", "⋯", "Ba chấm ngang giữa", "mo"),
        ("Spaces", "vdots", "⋮", "Ba chấm dọc", "mo"),
        ("Spaces", "ddots", "⋱", "Ba chấm chéo xuống", "mo"),
        ("Spaces", "iddots", "⋰", "Ba chấm chéo lên", "mo"),
        ("Spaces", "dprime", "″", "Hai phẩy trên", "mo"),
        ("Spaces", "trprime", "‴", "Ba phẩy trên", "mo"),
        ("Spaces", "qprime", "⁗", "Bốn phẩy trên", "mo"),
    ],
    templates: [
        ("number-set-R", "Symbols", "Tập số R", "\\mathbb{R}"),
        ("number-set-Z", "Symbols", "Tập số Z", "\\mathbb{Z}"),
        ("number-set-C", "Symbols", "Tập số C", "\\mathbb{C}"),
        ("number-set-Q", "Symbols", "Tập số Q", "\\mathbb{Q}"),
        ("number-set-N", "Symbols", "Tập số N", "\\mathbb{N}"),
        ("script-A", "Symbols", "Chữ viết tay A", "\\mathcal{A}"),
        ("script-B", "Symbols", "Chữ viết tay B", "\\mathcal{B}"),
        ("script-S", "Symbols", "Chữ viết tay S", "\\mathcal{S}"),
        ("script-R", "Symbols", "Chữ viết tay R", "\\mathcal{R}"),
        ("script-N", "Symbols", "Chữ viết tay N", "\\mathcal{N}"),
        ("fraktur-A", "Symbols", "Chữ Fraktur A", "\\mathfrak{A}"),
        ("fraktur-B", "Symbols", "Chữ Fraktur B", "\\mathfrak{B}"),
        ("fraktur-S", "Symbols", "Chữ Fraktur S", "\\mathfrak{S}"),
        ("fraktur-R", "Symbols", "Chữ Fraktur R", "\\mathfrak{R}"),
        ("fraktur-N", "Symbols", "Chữ Fraktur N", "\\mathfrak{N}"),
        ("paren-both", "Fences", "Ngoặc tròn", "\\left( #? \\right)"),
        ("paren-left", "Fences", "Ngoặc tròn trái", "\\left( #? \\right."),
        ("paren-right", "Fences", "Ngoặc tròn phải", "\\left. #? \\right)"),
        ("bracket-both", "Fences", "Ngoặc vuông", "\\left[ #? \\right]"),
        ("bracket-left", "Fences", "Ngoặc vuông trái", "\\left[ #? \\right."),
        ("bracket-right", "Fences", "Ngoặc vuông phải", "\\left. #? \\right]"),
        ("brace-both", "Fences", "Ngoặc nhọn", "\\left\\{ #? \\right\\}"),
        ("brace-left", "Fences", "Ngoặc nhọn trái", "\\left\\{ #? \\right."),
        ("brace-right", "Fences", "Ngoặc nhọn phải", "\\left. #? \\right\\}"),
        ("angle-both", "Fences", "Ngoặc góc", "\\left\\langle #? \\right\\rangle"),
        ("angle-left", "Fences", "Ngoặc góc trái", "\\left\\langle #? \\right."),
        ("angle-right", "Fences", "Ngoặc góc phải", "\\left. #? \\right\\rangle"),
        ("bar-both", "Fences", "Giá trị tuyệt đối", "\\left| #? \\right|"),
        ("bar-left", "Fences", "Giá trị tuyệt đối trái", "\\left| #? \\right."),
        ("bar-right", "Fences", "Giá trị tuyệt đối phải", "\\left. #? \\right|"),
        ("norm-both", "Fences", "Chuẩn", "\\left\\Vert #? \\right\\Vert"),
        ("norm-left", "Fences", "Chuẩn trái", "\\left\\Vert #? \\right."),
        ("norm-right", "Fences", "Chuẩn phải", "\\left. #? \\right\\Vert"),
        ("floor-both", "Fences", "Phần nguyên dưới", "\\left\\lfloor #? \\right\\rfloor"),
        ("floor-left", "Fences", "Phần nguyên dưới trái", "\\left\\lfloor #? \\right."),
        ("floor-right", "Fences", "Phần nguyên dưới phải", "\\left. #? \\right\\rfloor"),
        ("ceiling-both", "Fences", "Phần nguyên trên", "\\left\\lceil #? \\right\\rceil"),
        ("ceiling-left", "Fences", "Phần nguyên trên trái", "\\left\\lceil #? \\right."),
        ("ceiling-right", "Fences", "Phần nguyên trên phải", "\\left. #? \\right\\rceil"),
        ("bracket-paren", "Fences", "Nửa khoảng đóng trái mở phải", "\\left[ #? \\right)"),
        ("paren-bracket", "Fences", "Nửa khoảng mở trái đóng phải", "\\left( #? \\right]"),
        ("ket", "Fences", "Ket", "\\left| #? \\right\\rangle"),
        ("bra", "Fences", "Bra", "\\left\\langle #? \\right|"),
        ("bracket-left-left", "Fences", "Khoảng đóng trái mở phải vuông", "\\left[ #? \\right["),
        ("bracket-right-right", "Fences", "Khoảng mở trái đóng phải vuông", "\\left] #? \\right]"),
        ("bracket-open-open", "Fences", "Khoảng mở hai đầu vuông", "\\left] #? \\right["),
        ("double-bracket-both", "Fences", "Ngoặc vuông kép", "\\left\\llbracket #? \\right\\rrbracket"),
        ("double-bracket-left", "Fences", "Ngoặc vuông kép trái", "\\left\\llbracket #? \\right."),
        ("double-bracket-right", "Fences", "Ngoặc vuông kép phải", "\\left. #? \\right\\rrbracket"),
        ("overbrace", "Fences", "Ngoặc nhọn trên", "\\overbrace{#?}"),
        ("overbrace-label", "Fences", "Ngoặc nhọn trên có nhãn", "\\overbrace{#?}^{#?}"),
        ("underbrace", "Fences", "Ngoặc nhọn dưới", "\\underbrace{#?}"),
        ("underbrace-label", "Fences", "Ngoặc nhọn dưới có nhãn", "\\underbrace{#?}_{#?}"),
        ("overbracket", "Fences", "Ngoặc vuông trên", "\\overbracket{#?}"),
        ("overbracket-label", "Fences", "Ngoặc vuông trên có nhãn", "\\overbracket{#?}^{#?}"),
        ("underbracket", "Fences", "Ngoặc vuông dưới", "\\underbracket{#?}"),
        ("underbracket-label", "Fences", "Ngoặc vuông dưới có nhãn", "\\underbracket{#?}_{#?}"),
        ("wideparen", "Fences", "Cung trên", "\\wideparen{#?}"),
        ("underparen", "Fences", "Cung dưới", "\\underparen{#?}"),
        ("cases-two", "Fences", "Hệ hai nhánh (Ctrl+Shift+C)", "\\begin{cases} #? & #? \\\\ #? & #? \\end{cases}"),
        ("cases-three", "Fences", "Hệ ba nhánh", "\\begin{cases} #? & #? \\\\ #? & #? \\\\ #? & #? \\end{cases}"),
        ("evaluation", "Fences", "Thanh đánh giá", "\\left. #? \\right|_{#?}^{#?}"),
        ("bra-ket", "Fences", "Bra–ket", "\\left\\langle #? \\middle| #? \\right\\rangle"),
        ("frac", "Fractions", "Phân số (Ctrl+F hoặc /)", "\\frac{#?}{#?}"),
        ("tfrac", "Fractions", "Phân số nhỏ", "\\tfrac{#?}{#?}"),
        ("dfrac", "Fractions", "Phân số hiển thị", "\\dfrac{#?}{#?}"),
        ("sfrac", "Fractions", "Phân số chéo", "\\sfrac{#?}{#?}"),
        ("binom", "Fractions", "Nhị thức", "\\binom{#?}{#?}"),
        ("linear", "Fractions", "Phân số trên dòng", "{#?}/{#?}"),
        ("small-linear", "Fractions", "Phân số nhỏ trên dòng", "\\tsfrac{#?}{#?}"),
        ("sqrt", "Fractions", "Căn bậc hai (Ctrl+R)", "\\sqrt{#?}"),
        ("root", "Fractions", "Căn bậc n (Ctrl+Shift+R)", "\\sqrt[#?]{#?}"),
        ("root-fraction", "Fractions", "Căn của phân số", "\\sqrt{\\frac{#?}{#?}}"),
        ("small-root", "Fractions", "Căn nhỏ", "\\sqrt{\\tfrac{#?}{#?}}"),
        ("long-division", "Fractions", "Chia dài", "\\enclose{longdiv}{#?}"),
        ("long-division-quotient", "Fractions", "Chia dài có thương", "\\overset{#?}{\\enclose{longdiv}{#?}}"),
        ("sup", "Scripts", "Chỉ số trên (Ctrl+H hoặc ^)", "^{#?}"),
        ("sub", "Scripts", "Chỉ số dưới (Ctrl+L hoặc _)", "_{#?}"),
        ("subsup", "Scripts", "Chỉ số trên và dưới (Ctrl+J)", "_{#?}^{#?}"),
        ("pre-sup-post-none", "Scripts", "Chỉ số trái trên", "\\prescript{#?}{}{#?}"),
        ("pre-sup-post-sup", "Scripts", "Chỉ số trái trên và phải sup", "\\prescript{#?}{}{#?}^{#?}"),
        ("pre-sup-post-sub", "Scripts", "Chỉ số trái trên và phải sub", "\\prescript{#?}{}{#?}_{#?}"),
        ("pre-sup-post-both", "Scripts", "Chỉ số trái trên và phải both", "\\prescript{#?}{}{#?}_{#?}^{#?}"),
        ("pre-sub-post-none", "Scripts", "Chỉ số trái dưới", "\\prescript{}{#?}{#?}"),
        ("pre-sub-post-sup", "Scripts", "Chỉ số trái dưới và phải sup", "\\prescript{}{#?}{#?}^{#?}"),
        ("pre-sub-post-sub", "Scripts", "Chỉ số trái dưới và phải sub", "\\prescript{}{#?}{#?}_{#?}"),
        ("pre-sub-post-both", "Scripts", "Chỉ số trái dưới và phải both", "\\prescript{}{#?}{#?}_{#?}^{#?}"),
        ("pre-both-post-none", "Scripts", "Chỉ số trái trên/dưới", "\\prescript{#?}{#?}{#?}"),
        ("pre-both-post-sup", "Scripts", "Chỉ số trái trên/dưới và phải sup", "\\prescript{#?}{#?}{#?}^{#?}"),
        ("pre-both-post-sub", "Scripts", "Chỉ số trái trên/dưới và phải sub", "\\prescript{#?}{#?}{#?}_{#?}"),
        ("pre-both-post-both", "Scripts", "Chỉ số trái trên/dưới và phải both", "\\prescript{#?}{#?}{#?}_{#?}^{#?}"),
        ("lim", "Scripts", "Giới hạn (Ctrl+Shift+L)", "\\lim_{#? \\to #?}"),
        ("max", "Scripts", "Cực đại có điều kiện", "\\max_{#?}"),
        ("min", "Scripts", "Cực tiểu có điều kiện", "\\min_{#?}"),
        ("overset", "Scripts", "Nhãn trên", "\\overset{#?}{#?}"),
        ("underset", "Scripts", "Nhãn dưới", "\\underset{#?}{#?}"),
        ("overunderset", "Scripts", "Nhãn trên và dưới", "\\overset{#?}{\\underset{#?}{#?}}"),
        ("sum-side-lower", "Sums", "Tổng cận dưới bên cạnh", "\\sum\\nolimits_{#?} #?"),
        ("sum-side-upper", "Sums", "Tổng cận trên bên cạnh", "\\sum\\nolimits^{#?} #?"),
        ("sum-side-both", "Sums", "Tổng hai cận bên cạnh", "\\sum\\nolimits_{#?}^{#?} #?"),
        ("sum-stack-lower", "Sums", "Tổng cận dưới trên/dưới", "\\sum\\limits_{#?} #?"),
        ("sum-stack-upper", "Sums", "Tổng cận trên trên/dưới", "\\sum\\limits^{#?} #?"),
        ("sum-stack-both", "Sums", "Tổng hai cận trên/dưới", "\\sum\\limits_{#?}^{#?} #?"),
        ("int-side-lower", "Integrals", "Tích phân cận dưới bên cạnh", "\\int\\nolimits_{#?} #?"),
        ("int-side-upper", "Integrals", "Tích phân cận trên bên cạnh", "\\int\\nolimits^{#?} #?"),
        ("int-side-both", "Integrals", "Tích phân hai cận bên cạnh", "\\int\\nolimits_{#?}^{#?} #?"),
        ("int-stack-lower", "Integrals", "Tích phân cận dưới trên/dưới", "\\int\\limits_{#?} #?"),
        ("int-stack-upper", "Integrals", "Tích phân cận trên trên/dưới", "\\int\\limits^{#?} #?"),
        ("int-stack-both", "Integrals", "Tích phân hai cận trên/dưới", "\\int\\limits_{#?}^{#?} #?"),
        ("iint-side-lower", "Integrals", "Tích phân kép cận dưới bên cạnh", "\\iint\\nolimits_{#?} #?"),
        ("iint-side-upper", "Integrals", "Tích phân kép cận trên bên cạnh", "\\iint\\nolimits^{#?} #?"),
        ("iint-side-both", "Integrals", "Tích phân kép hai cận bên cạnh", "\\iint\\nolimits_{#?}^{#?} #?"),
        ("iint-stack-lower", "Integrals", "Tích phân kép cận dưới trên/dưới", "\\iint\\limits_{#?} #?"),
        ("iint-stack-upper", "Integrals", "Tích phân kép cận trên trên/dưới", "\\iint\\limits^{#?} #?"),
        ("iint-stack-both", "Integrals", "Tích phân kép hai cận trên/dưới", "\\iint\\limits_{#?}^{#?} #?"),
        ("iiint-side-lower", "Integrals", "Tích phân ba cận dưới bên cạnh", "\\iiint\\nolimits_{#?} #?"),
        ("iiint-side-upper", "Integrals", "Tích phân ba cận trên bên cạnh", "\\iiint\\nolimits^{#?} #?"),
        ("iiint-side-both", "Integrals", "Tích phân ba hai cận bên cạnh", "\\iiint\\nolimits_{#?}^{#?} #?"),
        ("iiint-stack-lower", "Integrals", "Tích phân ba cận dưới trên/dưới", "\\iiint\\limits_{#?} #?"),
        ("iiint-stack-upper", "Integrals", "Tích phân ba cận trên trên/dưới", "\\iiint\\limits^{#?} #?"),
        ("iiint-stack-both", "Integrals", "Tích phân ba hai cận trên/dưới", "\\iiint\\limits_{#?}^{#?} #?"),
        ("oint-side-lower", "Integrals", "Tích phân vòng cận dưới bên cạnh", "\\oint\\nolimits_{#?} #?"),
        ("oint-side-upper", "Integrals", "Tích phân vòng cận trên bên cạnh", "\\oint\\nolimits^{#?} #?"),
        ("oint-side-both", "Integrals", "Tích phân vòng hai cận bên cạnh", "\\oint\\nolimits_{#?}^{#?} #?"),
        ("oint-stack-lower", "Integrals", "Tích phân vòng cận dưới trên/dưới", "\\oint\\limits_{#?} #?"),
        ("oint-stack-upper", "Integrals", "Tích phân vòng cận trên trên/dưới", "\\oint\\limits^{#?} #?"),
        ("oint-stack-both", "Integrals", "Tích phân vòng hai cận trên/dưới", "\\oint\\limits_{#?}^{#?} #?"),
        ("oiint-side-lower", "Integrals", "Tích phân mặt kín cận dưới bên cạnh", "\\oiint\\nolimits_{#?} #?"),
        ("oiint-side-upper", "Integrals", "Tích phân mặt kín cận trên bên cạnh", "\\oiint\\nolimits^{#?} #?"),
        ("oiint-side-both", "Integrals", "Tích phân mặt kín hai cận bên cạnh", "\\oiint\\nolimits_{#?}^{#?} #?"),
        ("oiint-stack-lower", "Integrals", "Tích phân mặt kín cận dưới trên/dưới", "\\oiint\\limits_{#?} #?"),
        ("oiint-stack-upper", "Integrals", "Tích phân mặt kín cận trên trên/dưới", "\\oiint\\limits^{#?} #?"),
        ("oiint-stack-both", "Integrals", "Tích phân mặt kín hai cận trên/dưới", "\\oiint\\limits_{#?}^{#?} #?"),
        ("oiiint-side-lower", "Integrals", "Tích phân khối kín cận dưới bên cạnh", "\\oiiint\\nolimits_{#?} #?"),
        ("oiiint-side-upper", "Integrals", "Tích phân khối kín cận trên bên cạnh", "\\oiiint\\nolimits^{#?} #?"),
        ("oiiint-side-both", "Integrals", "Tích phân khối kín hai cận bên cạnh", "\\oiiint\\nolimits_{#?}^{#?} #?"),
        ("oiiint-stack-lower", "Integrals", "Tích phân khối kín cận dưới trên/dưới", "\\oiiint\\limits_{#?} #?"),
        ("oiiint-stack-upper", "Integrals", "Tích phân khối kín cận trên trên/dưới", "\\oiiint\\limits^{#?} #?"),
        ("oiiint-stack-both", "Integrals", "Tích phân khối kín hai cận trên/dưới", "\\oiiint\\limits_{#?}^{#?} #?"),
        ("varointclockwise-side-lower", "Integrals", "Tích phân vòng thuận chiều cận dưới bên cạnh", "\\varointclockwise\\nolimits_{#?} #?"),
        ("varointclockwise-side-upper", "Integrals", "Tích phân vòng thuận chiều cận trên bên cạnh", "\\varointclockwise\\nolimits^{#?} #?"),
        ("varointclockwise-side-both", "Integrals", "Tích phân vòng thuận chiều hai cận bên cạnh", "\\varointclockwise\\nolimits_{#?}^{#?} #?"),
        ("varointclockwise-stack-lower", "Integrals", "Tích phân vòng thuận chiều cận dưới trên/dưới", "\\varointclockwise\\limits_{#?} #?"),
        ("varointclockwise-stack-upper", "Integrals", "Tích phân vòng thuận chiều cận trên trên/dưới", "\\varointclockwise\\limits^{#?} #?"),
        ("varointclockwise-stack-both", "Integrals", "Tích phân vòng thuận chiều hai cận trên/dưới", "\\varointclockwise\\limits_{#?}^{#?} #?"),
        ("ointctrclockwise-side-lower", "Integrals", "Tích phân vòng ngược chiều cận dưới bên cạnh", "\\ointctrclockwise\\nolimits_{#?} #?"),
        ("ointctrclockwise-side-upper", "Integrals", "Tích phân vòng ngược chiều cận trên bên cạnh", "\\ointctrclockwise\\nolimits^{#?} #?"),
        ("ointctrclockwise-side-both", "Integrals", "Tích phân vòng ngược chiều hai cận bên cạnh", "\\ointctrclockwise\\nolimits_{#?}^{#?} #?"),
        ("ointctrclockwise-stack-lower", "Integrals", "Tích phân vòng ngược chiều cận dưới trên/dưới", "\\ointctrclockwise\\limits_{#?} #?"),
        ("ointctrclockwise-stack-upper", "Integrals", "Tích phân vòng ngược chiều cận trên trên/dưới", "\\ointctrclockwise\\limits^{#?} #?"),
        ("ointctrclockwise-stack-both", "Integrals", "Tích phân vòng ngược chiều hai cận trên/dưới", "\\ointctrclockwise\\limits_{#?}^{#?} #?"),
        ("int-differential", "Integrals", "Tích phân với vi phân (Ctrl+I)", "\\int #? \\, d#?"),
        ("definite-differential", "Integrals", "Tích phân xác định (Ctrl+Shift+I)", "\\int\\limits_{#?}^{#?} #? \\, d#?"),
        ("prod-side-lower", "BigOperators", "Tích cận dưới bên cạnh", "\\prod\\nolimits_{#?} #?"),
        ("prod-side-upper", "BigOperators", "Tích cận trên bên cạnh", "\\prod\\nolimits^{#?} #?"),
        ("prod-side-both", "BigOperators", "Tích hai cận bên cạnh", "\\prod\\nolimits_{#?}^{#?} #?"),
        ("prod-stack-lower", "BigOperators", "Tích cận dưới trên/dưới", "\\prod\\limits_{#?} #?"),
        ("prod-stack-upper", "BigOperators", "Tích cận trên trên/dưới", "\\prod\\limits^{#?} #?"),
        ("prod-stack-both", "BigOperators", "Tích hai cận trên/dưới", "\\prod\\limits_{#?}^{#?} #?"),
        ("coprod-side-lower", "BigOperators", "Đồng tích cận dưới bên cạnh", "\\coprod\\nolimits_{#?} #?"),
        ("coprod-side-upper", "BigOperators", "Đồng tích cận trên bên cạnh", "\\coprod\\nolimits^{#?} #?"),
        ("coprod-side-both", "BigOperators", "Đồng tích hai cận bên cạnh", "\\coprod\\nolimits_{#?}^{#?} #?"),
        ("coprod-stack-lower", "BigOperators", "Đồng tích cận dưới trên/dưới", "\\coprod\\limits_{#?} #?"),
        ("coprod-stack-upper", "BigOperators", "Đồng tích cận trên trên/dưới", "\\coprod\\limits^{#?} #?"),
        ("coprod-stack-both", "BigOperators", "Đồng tích hai cận trên/dưới", "\\coprod\\limits_{#?}^{#?} #?"),
        ("bigcap-side-lower", "BigOperators", "Giao lớn cận dưới bên cạnh", "\\bigcap\\nolimits_{#?} #?"),
        ("bigcap-side-upper", "BigOperators", "Giao lớn cận trên bên cạnh", "\\bigcap\\nolimits^{#?} #?"),
        ("bigcap-side-both", "BigOperators", "Giao lớn hai cận bên cạnh", "\\bigcap\\nolimits_{#?}^{#?} #?"),
        ("bigcap-stack-lower", "BigOperators", "Giao lớn cận dưới trên/dưới", "\\bigcap\\limits_{#?} #?"),
        ("bigcap-stack-upper", "BigOperators", "Giao lớn cận trên trên/dưới", "\\bigcap\\limits^{#?} #?"),
        ("bigcap-stack-both", "BigOperators", "Giao lớn hai cận trên/dưới", "\\bigcap\\limits_{#?}^{#?} #?"),
        ("bigcup-side-lower", "BigOperators", "Hợp lớn cận dưới bên cạnh", "\\bigcup\\nolimits_{#?} #?"),
        ("bigcup-side-upper", "BigOperators", "Hợp lớn cận trên bên cạnh", "\\bigcup\\nolimits^{#?} #?"),
        ("bigcup-side-both", "BigOperators", "Hợp lớn hai cận bên cạnh", "\\bigcup\\nolimits_{#?}^{#?} #?"),
        ("bigcup-stack-lower", "BigOperators", "Hợp lớn cận dưới trên/dưới", "\\bigcup\\limits_{#?} #?"),
        ("bigcup-stack-upper", "BigOperators", "Hợp lớn cận trên trên/dưới", "\\bigcup\\limits^{#?} #?"),
        ("bigcup-stack-both", "BigOperators", "Hợp lớn hai cận trên/dưới", "\\bigcup\\limits_{#?}^{#?} #?"),
        ("bigwedge-side-lower", "BigOperators", "Hội lớn cận dưới bên cạnh", "\\bigwedge\\nolimits_{#?} #?"),
        ("bigwedge-side-upper", "BigOperators", "Hội lớn cận trên bên cạnh", "\\bigwedge\\nolimits^{#?} #?"),
        ("bigwedge-side-both", "BigOperators", "Hội lớn hai cận bên cạnh", "\\bigwedge\\nolimits_{#?}^{#?} #?"),
        ("bigwedge-stack-lower", "BigOperators", "Hội lớn cận dưới trên/dưới", "\\bigwedge\\limits_{#?} #?"),
        ("bigwedge-stack-upper", "BigOperators", "Hội lớn cận trên trên/dưới", "\\bigwedge\\limits^{#?} #?"),
        ("bigwedge-stack-both", "BigOperators", "Hội lớn hai cận trên/dưới", "\\bigwedge\\limits_{#?}^{#?} #?"),
        ("bigvee-side-lower", "BigOperators", "Tuyển lớn cận dưới bên cạnh", "\\bigvee\\nolimits_{#?} #?"),
        ("bigvee-side-upper", "BigOperators", "Tuyển lớn cận trên bên cạnh", "\\bigvee\\nolimits^{#?} #?"),
        ("bigvee-side-both", "BigOperators", "Tuyển lớn hai cận bên cạnh", "\\bigvee\\nolimits_{#?}^{#?} #?"),
        ("bigvee-stack-lower", "BigOperators", "Tuyển lớn cận dưới trên/dưới", "\\bigvee\\limits_{#?} #?"),
        ("bigvee-stack-upper", "BigOperators", "Tuyển lớn cận trên trên/dưới", "\\bigvee\\limits^{#?} #?"),
        ("bigvee-stack-both", "BigOperators", "Tuyển lớn hai cận trên/dưới", "\\bigvee\\limits_{#?}^{#?} #?"),
        ("bigotimes-side-lower", "BigOperators", "Tích tensor lớn cận dưới bên cạnh", "\\bigotimes\\nolimits_{#?} #?"),
        ("bigotimes-side-upper", "BigOperators", "Tích tensor lớn cận trên bên cạnh", "\\bigotimes\\nolimits^{#?} #?"),
        ("bigotimes-side-both", "BigOperators", "Tích tensor lớn hai cận bên cạnh", "\\bigotimes\\nolimits_{#?}^{#?} #?"),
        ("bigotimes-stack-lower", "BigOperators", "Tích tensor lớn cận dưới trên/dưới", "\\bigotimes\\limits_{#?} #?"),
        ("bigotimes-stack-upper", "BigOperators", "Tích tensor lớn cận trên trên/dưới", "\\bigotimes\\limits^{#?} #?"),
        ("bigotimes-stack-both", "BigOperators", "Tích tensor lớn hai cận trên/dưới", "\\bigotimes\\limits_{#?}^{#?} #?"),
        ("bigoplus-side-lower", "BigOperators", "Tổng trực tiếp lớn cận dưới bên cạnh", "\\bigoplus\\nolimits_{#?} #?"),
        ("bigoplus-side-upper", "BigOperators", "Tổng trực tiếp lớn cận trên bên cạnh", "\\bigoplus\\nolimits^{#?} #?"),
        ("bigoplus-side-both", "BigOperators", "Tổng trực tiếp lớn hai cận bên cạnh", "\\bigoplus\\nolimits_{#?}^{#?} #?"),
        ("bigoplus-stack-lower", "BigOperators", "Tổng trực tiếp lớn cận dưới trên/dưới", "\\bigoplus\\limits_{#?} #?"),
        ("bigoplus-stack-upper", "BigOperators", "Tổng trực tiếp lớn cận trên trên/dưới", "\\bigoplus\\limits^{#?} #?"),
        ("bigoplus-stack-both", "BigOperators", "Tổng trực tiếp lớn hai cận trên/dưới", "\\bigoplus\\limits_{#?}^{#?} #?"),
        ("bigodot-side-lower", "BigOperators", "Tích chấm lớn cận dưới bên cạnh", "\\bigodot\\nolimits_{#?} #?"),
        ("bigodot-side-upper", "BigOperators", "Tích chấm lớn cận trên bên cạnh", "\\bigodot\\nolimits^{#?} #?"),
        ("bigodot-side-both", "BigOperators", "Tích chấm lớn hai cận bên cạnh", "\\bigodot\\nolimits_{#?}^{#?} #?"),
        ("bigodot-stack-lower", "BigOperators", "Tích chấm lớn cận dưới trên/dưới", "\\bigodot\\limits_{#?} #?"),
        ("bigodot-stack-upper", "BigOperators", "Tích chấm lớn cận trên trên/dưới", "\\bigodot\\limits^{#?} #?"),
        ("bigodot-stack-both", "BigOperators", "Tích chấm lớn hai cận trên/dưới", "\\bigodot\\limits_{#?}^{#?} #?"),
        ("dot", "Embellishments", "Một chấm trên", "\\dot{#?}"),
        ("ddot", "Embellishments", "Hai chấm trên", "\\ddot{#?}"),
        ("dddot", "Embellishments", "Ba chấm trên", "\\dddot{#?}"),
        ("ddddot", "Embellishments", "Bốn chấm trên", "\\ddddot{#?}"),
        ("hat", "Embellishments", "Dấu mũ", "\\hat{#?}"),
        ("widehat", "Embellishments", "Mũ rộng", "\\widehat{#?}"),
        ("tilde", "Embellishments", "Dấu sóng", "\\tilde{#?}"),
        ("widetilde", "Embellishments", "Sóng rộng", "\\widetilde{#?}"),
        ("check", "Embellishments", "Dấu háček", "\\check{#?}"),
        ("breve", "Embellishments", "Dấu breve", "\\breve{#?}"),
        ("mathring", "Embellishments", "Vòng tròn trên", "\\mathring{#?}"),
        ("vec", "Embellishments", "Vector phải (Ctrl+Alt+V)", "\\vec{#?}"),
        ("overleftarrow", "Embellishments", "Vector trái", "\\overleftarrow{#?}"),
        ("overleftrightarrow", "Embellishments", "Vector hai chiều", "\\overleftrightarrow{#?}"),
        ("underdot", "Embellishments", "Một chấm dưới", "\\underdot{#?}"),
        ("underddot", "Embellishments", "Hai chấm dưới", "\\underddot{#?}"),
        ("underdddot", "Embellishments", "Ba chấm dưới", "\\underdddot{#?}"),
        ("underddddot", "Embellishments", "Bốn chấm dưới", "\\underddddot{#?}"),
        ("underleftarrow", "Embellishments", "Mũi tên dưới trái", "\\underleftarrow{#?}"),
        ("underrightarrow", "Embellishments", "Mũi tên dưới phải", "\\underrightarrow{#?}"),
        ("underleftrightarrow", "Embellishments", "Mũi tên dưới hai chiều", "\\underleftrightarrow{#?}"),
        ("underparen", "Embellishments", "Cung dưới", "\\underparen{#?}"),
        ("wideparen", "Embellishments", "Cung trên", "\\wideparen{#?}"),
        ("cancel", "Embellishments", "Gạch chéo lên", "\\cancel{#?}"),
        ("bcancel", "Embellishments", "Gạch chéo xuống", "\\bcancel{#?}"),
        ("xcancel", "Embellishments", "Gạch chéo chữ X", "\\xcancel{#?}"),
        ("strike-horizontal", "Embellishments", "Gạch ngang", "\\enclose{horizontalstrike}{#?}"),
        ("strike-vertical", "Embellishments", "Gạch đứng", "\\enclose{verticalstrike}{#?}"),
        ("strike-horizontal-vertical", "Embellishments", "Gạch chữ thập", "\\enclose{horizontalstrike verticalstrike}{#?}"),
        ("prime", "Embellishments", "Một phẩy trên", "{#?}'"),
        ("double-prime", "Embellishments", "Hai phẩy trên", "{#?}''"),
        ("triple-prime", "Embellishments", "Ba phẩy trên", "{#?}'''"),
        ("quadruple-prime", "Embellishments", "Bốn phẩy trên", "{#?}^{⁗}"),
        ("left-prime", "Embellishments", "Phẩy trên trái", "\\prescript{\\prime}{}{#?}"),
        ("back-prime", "Embellishments", "Phẩy trên ngược", "{#?}^{\\backprime}"),
        ("underhat", "Embellishments", "Mũ dưới", "\\underhat{#?}"),
        ("undertilde", "Embellishments", "Sóng dưới", "\\undertilde{#?}"),
        ("undercheck", "Embellishments", "Háček dưới", "\\undercheck{#?}"),
        ("underbreve", "Embellishments", "Breve dưới", "\\underbreve{#?}"),
        ("underring", "Embellishments", "Vòng tròn dưới", "\\underring{#?}"),
        ("overleftharpoon", "Embellishments", "Móc trên trái", "\\overleftharpoon{#?}"),
        ("overrightharpoon", "Embellishments", "Móc trên phải", "\\overrightharpoon{#?}"),
        ("underleftharpoon", "Embellishments", "Móc dưới trái", "\\underleftharpoon{#?}"),
        ("underrightharpoon", "Embellishments", "Móc dưới phải", "\\underrightharpoon{#?}"),
        ("over-acute", "Embellishments", "Dấu acute trên", "\\overaccent{ˊ}{#?}"),
        ("under-acute", "Embellishments", "Dấu acute dưới", "\\underaccent{ˊ}{#?}"),
        ("over-grave", "Embellishments", "Dấu grave trên", "\\overaccent{ˋ}{#?}"),
        ("under-grave", "Embellishments", "Dấu grave dưới", "\\underaccent{ˋ}{#?}"),
        ("over-arrow-both", "Embellishments", "Dấu arrow-both trên", "\\overaccent{↔}{#?}"),
        ("under-arrow-both", "Embellishments", "Dấu arrow-both dưới", "\\underaccent{↔}{#?}"),
        ("over-double-arrow-both", "Embellishments", "Dấu double-arrow-both trên", "\\overaccent{⇔}{#?}"),
        ("under-double-arrow-both", "Embellishments", "Dấu double-arrow-both dưới", "\\underaccent{⇔}{#?}"),
        ("over-arrow-left", "Embellishments", "Dấu arrow-left trên", "\\overaccent{←}{#?}"),
        ("under-arrow-left", "Embellishments", "Dấu arrow-left dưới", "\\underaccent{←}{#?}"),
        ("over-arrow-right", "Embellishments", "Dấu arrow-right trên", "\\overaccent{→}{#?}"),
        ("under-arrow-right", "Embellishments", "Dấu arrow-right dưới", "\\underaccent{→}{#?}"),
        ("over-harpoon-left", "Embellishments", "Dấu harpoon-left trên", "\\overaccent{↼}{#?}"),
        ("under-harpoon-left", "Embellishments", "Dấu harpoon-left dưới", "\\underaccent{↼}{#?}"),
        ("over-harpoon-right", "Embellishments", "Dấu harpoon-right trên", "\\overaccent{⇀}{#?}"),
        ("under-harpoon-right", "Embellishments", "Dấu harpoon-right dưới", "\\underaccent{⇀}{#?}"),
        ("overline", "Bars", "Gạch trên", "\\overline{#?}"),
        ("underline", "Bars", "Gạch dưới", "\\underline{#?}"),
        ("doubleoverline", "Bars", "Gạch kép trên", "\\doubleoverline{#?}"),
        ("doubleunderline", "Bars", "Gạch kép dưới", "\\doubleunderline{#?}"),
        ("widehat", "Bars", "Mũ rộng", "\\widehat{#?}"),
        ("widetilde", "Bars", "Sóng rộng", "\\widetilde{#?}"),
        ("wideparen", "Bars", "Cung trên", "\\wideparen{#?}"),
        ("underparen", "Bars", "Cung dưới", "\\underparen{#?}"),
        ("overleftarrow", "Bars", "Mũi tên trên trái", "\\overleftarrow{#?}"),
        ("overrightarrow", "Bars", "Mũi tên trên phải", "\\overrightarrow{#?}"),
        ("overleftrightarrow", "Bars", "Mũi tên trên hai chiều", "\\overleftrightarrow{#?}"),
        ("underleftarrow", "Bars", "Mũi tên dưới trái", "\\underleftarrow{#?}"),
        ("underrightarrow", "Bars", "Mũi tên dưới phải", "\\underrightarrow{#?}"),
        ("underleftrightarrow", "Bars", "Mũi tên dưới hai chiều", "\\underleftrightarrow{#?}"),
        ("overbrace-label", "Bars", "Ngoặc nhọn trên có nhãn", "\\overbrace{#?}^{#?}"),
        ("underbrace-label", "Bars", "Ngoặc nhọn dưới có nhãn", "\\underbrace{#?}_{#?}"),
        ("overbracket-label", "Bars", "Ngoặc vuông trên có nhãn", "\\overbracket{#?}^{#?}"),
        ("underbracket-label", "Bars", "Ngoặc vuông dưới có nhãn", "\\underbracket{#?}_{#?}"),
        ("overline-label", "Bars", "Gạch trên có nhãn", "\\overset{#?}{\\overline{#?}}"),
        ("underline-label", "Bars", "Gạch dưới có nhãn", "\\underset{#?}{\\underline{#?}}"),
        ("doubleoverline-label", "Bars", "Gạch kép trên có nhãn", "\\overset{#?}{\\doubleoverline{#?}}"),
        ("doubleunderline-label", "Bars", "Gạch kép dưới có nhãn", "\\underset{#?}{\\doubleunderline{#?}}"),
        ("strike-horizontalstrike", "Bars", "Gạch ngang giữa", "\\enclose{horizontalstrike}{#?}"),
        ("strike-updiagonalstrike", "Bars", "Gạch chéo lên", "\\enclose{updiagonalstrike}{#?}"),
        ("strike-downdiagonalstrike", "Bars", "Gạch chéo xuống", "\\enclose{downdiagonalstrike}{#?}"),
        ("strike-updiagonalstrike-downdiagonalstrike", "Bars", "Gạch chữ X", "\\enclose{updiagonalstrike downdiagonalstrike}{#?}"),
        ("rightarrow-top", "LabeledArrows", "Mũi tên phải nhãn trên", "\\overset{#?}{\\rightarrow}"),
        ("rightarrow-bottom", "LabeledArrows", "Mũi tên phải nhãn dưới", "\\underset{#?}{\\rightarrow}"),
        ("rightarrow-both", "LabeledArrows", "Mũi tên phải nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\rightarrow}}"),
        ("leftarrow-top", "LabeledArrows", "Mũi tên trái nhãn trên", "\\overset{#?}{\\leftarrow}"),
        ("leftarrow-bottom", "LabeledArrows", "Mũi tên trái nhãn dưới", "\\underset{#?}{\\leftarrow}"),
        ("leftarrow-both", "LabeledArrows", "Mũi tên trái nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\leftarrow}}"),
        ("leftrightarrow-top", "LabeledArrows", "Mũi tên hai chiều nhãn trên", "\\overset{#?}{\\leftrightarrow}"),
        ("leftrightarrow-bottom", "LabeledArrows", "Mũi tên hai chiều nhãn dưới", "\\underset{#?}{\\leftrightarrow}"),
        ("leftrightarrow-both", "LabeledArrows", "Mũi tên hai chiều nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\leftrightarrow}}"),
        ("Rightarrow-top", "LabeledArrows", "Mũi tên phải kép nhãn trên", "\\overset{#?}{\\Rightarrow}"),
        ("Rightarrow-bottom", "LabeledArrows", "Mũi tên phải kép nhãn dưới", "\\underset{#?}{\\Rightarrow}"),
        ("Rightarrow-both", "LabeledArrows", "Mũi tên phải kép nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\Rightarrow}}"),
        ("Leftarrow-top", "LabeledArrows", "Mũi tên trái kép nhãn trên", "\\overset{#?}{\\Leftarrow}"),
        ("Leftarrow-bottom", "LabeledArrows", "Mũi tên trái kép nhãn dưới", "\\underset{#?}{\\Leftarrow}"),
        ("Leftarrow-both", "LabeledArrows", "Mũi tên trái kép nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\Leftarrow}}"),
        ("Leftrightarrow-top", "LabeledArrows", "Mũi tên hai chiều kép nhãn trên", "\\overset{#?}{\\Leftrightarrow}"),
        ("Leftrightarrow-bottom", "LabeledArrows", "Mũi tên hai chiều kép nhãn dưới", "\\underset{#?}{\\Leftrightarrow}"),
        ("Leftrightarrow-both", "LabeledArrows", "Mũi tên hai chiều kép nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\Leftrightarrow}}"),
        ("rightharpoonup-top", "LabeledArrows", "Mũi tên móc phải nhãn trên", "\\overset{#?}{\\rightharpoonup}"),
        ("rightharpoonup-bottom", "LabeledArrows", "Mũi tên móc phải nhãn dưới", "\\underset{#?}{\\rightharpoonup}"),
        ("rightharpoonup-both", "LabeledArrows", "Mũi tên móc phải nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\rightharpoonup}}"),
        ("leftharpoonup-top", "LabeledArrows", "Mũi tên móc trái nhãn trên", "\\overset{#?}{\\leftharpoonup}"),
        ("leftharpoonup-bottom", "LabeledArrows", "Mũi tên móc trái nhãn dưới", "\\underset{#?}{\\leftharpoonup}"),
        ("leftharpoonup-both", "LabeledArrows", "Mũi tên móc trái nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\leftharpoonup}}"),
        ("rightleftharpoons-top", "LabeledArrows", "Mũi tên cân bằng thuận nhãn trên", "\\overset{#?}{\\rightleftharpoons}"),
        ("rightleftharpoons-bottom", "LabeledArrows", "Mũi tên cân bằng thuận nhãn dưới", "\\underset{#?}{\\rightleftharpoons}"),
        ("rightleftharpoons-both", "LabeledArrows", "Mũi tên cân bằng thuận nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\rightleftharpoons}}"),
        ("leftrightharpoons-top", "LabeledArrows", "Mũi tên cân bằng nghịch nhãn trên", "\\overset{#?}{\\leftrightharpoons}"),
        ("leftrightharpoons-bottom", "LabeledArrows", "Mũi tên cân bằng nghịch nhãn dưới", "\\underset{#?}{\\leftrightharpoons}"),
        ("leftrightharpoons-both", "LabeledArrows", "Mũi tên cân bằng nghịch nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\leftrightharpoons}}"),
        ("mapsto-top", "LabeledArrows", "Mũi tên ánh xạ nhãn trên", "\\overset{#?}{\\mapsto}"),
        ("mapsto-bottom", "LabeledArrows", "Mũi tên ánh xạ nhãn dưới", "\\underset{#?}{\\mapsto}"),
        ("mapsto-both", "LabeledArrows", "Mũi tên ánh xạ nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\mapsto}}"),
        ("rightleftarrows-top", "LabeledArrows", "Mũi tên hai mũi tên nhãn trên", "\\overset{#?}{\\rightleftarrows}"),
        ("rightleftarrows-bottom", "LabeledArrows", "Mũi tên hai mũi tên nhãn dưới", "\\underset{#?}{\\rightleftarrows}"),
        ("rightleftarrows-both", "LabeledArrows", "Mũi tên hai mũi tên nhãn trên/dưới", "\\overset{#?}{\\underset{#?}{\\rightleftarrows}}"),
        ("plain-1x2", "Matrices", "Ma trận 1×2", "\\begin{matrix} #? & #? \\end{matrix}"),
        ("plain-2x1", "Matrices", "Ma trận 2×1", "\\begin{matrix} #? \\\\ #? \\end{matrix}"),
        ("plain-1x3", "Matrices", "Ma trận 1×3", "\\begin{matrix} #? & #? & #? \\end{matrix}"),
        ("plain-3x1", "Matrices", "Ma trận 3×1", "\\begin{matrix} #? \\\\ #? \\\\ #? \\end{matrix}"),
        ("plain-2x2", "Matrices", "Ma trận 2×2", "\\begin{matrix} #? & #? \\\\ #? & #? \\end{matrix}"),
        ("plain-3x3", "Matrices", "Ma trận 3×3", "\\begin{matrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{matrix}"),
        ("plain-2x3", "Matrices", "Ma trận 2×3", "\\begin{matrix} #? & #? & #? \\\\ #? & #? & #? \\end{matrix}"),
        ("plain-3x2", "Matrices", "Ma trận 3×2", "\\begin{matrix} #? & #? \\\\ #? & #? \\\\ #? & #? \\end{matrix}"),
        ("plain-1x4", "Matrices", "Ma trận 1×4", "\\begin{matrix} #? & #? & #? & #? \\end{matrix}"),
        ("plain-4x1", "Matrices", "Ma trận 4×1", "\\begin{matrix} #? \\\\ #? \\\\ #? \\\\ #? \\end{matrix}"),
        ("plain-4x4", "Matrices", "Ma trận 4×4", "\\begin{matrix} #? & #? & #? & #? \\\\ #? & #? & #? & #? \\\\ #? & #? & #? & #? \\\\ #? & #? & #? & #? \\end{matrix}"),
        ("pmatrix2", "Matrices", "Ngoặc tròn 2×2", "\\begin{pmatrix} #? & #? \\\\ #? & #? \\end{pmatrix}"),
        ("pmatrix3", "Matrices", "Ngoặc tròn 3×3", "\\begin{pmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{pmatrix}"),
        ("bmatrix2", "Matrices", "Ngoặc vuông 2×2", "\\begin{bmatrix} #? & #? \\\\ #? & #? \\end{bmatrix}"),
        ("bmatrix3", "Matrices", "Ngoặc vuông 3×3", "\\begin{bmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{bmatrix}"),
        ("Bmatrix2", "Matrices", "Ngoặc nhọn 2×2", "\\begin{Bmatrix} #? & #? \\\\ #? & #? \\end{Bmatrix}"),
        ("Bmatrix3", "Matrices", "Ngoặc nhọn 3×3", "\\begin{Bmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{Bmatrix}"),
        ("vmatrix2", "Matrices", "Định thức 2×2", "\\begin{vmatrix} #? & #? \\\\ #? & #? \\end{vmatrix}"),
        ("vmatrix3", "Matrices", "Định thức 3×3", "\\begin{vmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{vmatrix}"),
        ("Vmatrix2", "Matrices", "Vạch kép 2×2", "\\begin{Vmatrix} #? & #? \\\\ #? & #? \\end{Vmatrix}"),
        ("Vmatrix3", "Matrices", "Vạch kép 3×3", "\\begin{Vmatrix} #? & #? & #? \\\\ #? & #? & #? \\\\ #? & #? & #? \\end{Vmatrix}"),
        ("dots-row", "Matrices", "Hàng tiếp diễn", "\\begin{matrix} #? & \\cdots & #? \\end{matrix}"),
        ("dots-column", "Matrices", "Cột tiếp diễn", "\\begin{matrix} #? \\\\ \\vdots \\\\ #? \\end{matrix}"),
        ("dots-matrix", "Matrices", "Ma trận tiếp diễn", "\\begin{matrix} #? & \\cdots & #? \\\\ \\vdots & \\ddots & \\vdots \\\\ #? & \\cdots & #? \\end{matrix}"),
        ("aligned", "Matrices", "Hệ căn theo dấu bằng", "\\begin{aligned} #? &= #? \\\\ #? &= #? \\end{aligned}"),
        ("gathered", "Matrices", "Hệ căn giữa", "\\begin{gathered} #? \\\\ #? \\end{gathered}"),
        ("top-left", "Boxes", "Khung góc trên trái", "\\enclose{top left}{#?}"),
        ("top-right", "Boxes", "Khung góc trên phải", "\\enclose{top right}{#?}"),
        ("bottom-left", "Boxes", "Khung góc dưới trái", "\\enclose{bottom left}{#?}"),
        ("bottom-right", "Boxes", "Khung góc dưới phải", "\\enclose{bottom right}{#?}"),
        ("box", "Boxes", "Khung kín", "\\enclose{box}{#?}"),
        ("roundedbox", "Boxes", "Khung bo góc", "\\enclose{roundedbox}{#?}"),
        ("top-bottom-left", "Boxes", "Khung hở phải", "\\enclose{top bottom left}{#?}"),
        ("top-bottom-right", "Boxes", "Khung hở trái", "\\enclose{top bottom right}{#?}"),
        ("top-left-right", "Boxes", "Khung hở dưới", "\\enclose{top left right}{#?}"),
        ("bottom-left-right", "Boxes", "Khung hở trên", "\\enclose{bottom left right}{#?}"),
        ("space-0.083333", "Spaces", "Khoảng 1 điểm", "\\hspace{0.083333em}"),
        ("space-0.166667", "Spaces", "Khoảng mảnh", "\\hspace{0.166667em}"),
        ("space-0.222222", "Spaces", "Khoảng vừa", "\\hspace{0.222222em}"),
        ("space-0.277778", "Spaces", "Khoảng dày", "\\hspace{0.277778em}"),
        ("space-1", "Spaces", "Khoảng em", "\\hspace{1em}"),
        ("space-2", "Spaces", "Khoảng hai em", "\\hspace{2em}"),
    ],
    commands: [
        ("frac", "Phân số", "\\frac{#?}{#?}"),
        ("dfrac", "Phân số hiển thị", "\\dfrac{#?}{#?}"),
        ("tfrac", "Phân số nhỏ", "\\tfrac{#?}{#?}"),
        ("sfrac", "Phân số chéo", "\\sfrac{#?}{#?}"),
        ("atopfrac", "Phân số không gạch", "\\atopfrac{#?}{#?}"),
        ("sqrt", "Căn bậc hai", "\\sqrt{#?}"),
        ("binom", "Nhị thức", "\\binom{#?}{#?}"),
        ("prescript", "Chỉ số trái", "\\prescript{#?}{#?}{#?}"),
        ("overset", "Nhãn trên", "\\overset{#?}{#?}"),
        ("underset", "Nhãn dưới", "\\underset{#?}{#?}"),
        ("enclose", "Khung", "\\enclose{box}{#?}"),
        ("boxed", "Khung kín", "\\boxed{#?}"),
        ("mathbb", "Chữ tập số", "\\mathbb{#?}"),
        ("mathfrak", "Chữ Fraktur", "\\mathfrak{#?}"),
        ("mathcal", "Chữ viết tay", "\\mathcal{#?}"),
        ("hspace", "Khoảng trắng", "\\hspace{1em}"),
        ("overbrace", "Ngoặc nhọn trên", "\\overbrace{#?}"),
        ("underbrace", "Ngoặc nhọn dưới", "\\underbrace{#?}"),
        ("overbracket", "Ngoặc vuông trên", "\\overbracket{#?}"),
        ("underbracket", "Ngoặc vuông dưới", "\\underbracket{#?}"),
        ("wideparen", "Cung trên", "\\wideparen{#?}"),
        ("underparen", "Cung dưới", "\\underparen{#?}"),
        ("tsfrac", "Phân số nhỏ trên dòng", "\\tsfrac{#?}{#?}"),
        ("dot", "Một chấm trên", "\\dot{#?}"),
        ("ddot", "Hai chấm trên", "\\ddot{#?}"),
        ("dddot", "Ba chấm trên", "\\dddot{#?}"),
        ("ddddot", "Bốn chấm trên", "\\ddddot{#?}"),
        ("hat", "Dấu mũ", "\\hat{#?}"),
        ("widehat", "Mũ rộng", "\\widehat{#?}"),
        ("tilde", "Dấu sóng", "\\tilde{#?}"),
        ("widetilde", "Sóng rộng", "\\widetilde{#?}"),
        ("check", "Dấu háček", "\\check{#?}"),
        ("breve", "Dấu breve", "\\breve{#?}"),
        ("mathring", "Vòng tròn trên", "\\mathring{#?}"),
        ("vec", "Vector phải (Ctrl+Alt+V)", "\\vec{#?}"),
        ("overleftarrow", "Vector trái", "\\overleftarrow{#?}"),
        ("overleftrightarrow", "Vector hai chiều", "\\overleftrightarrow{#?}"),
        ("underdot", "Một chấm dưới", "\\underdot{#?}"),
        ("underddot", "Hai chấm dưới", "\\underddot{#?}"),
        ("underdddot", "Ba chấm dưới", "\\underdddot{#?}"),
        ("underddddot", "Bốn chấm dưới", "\\underddddot{#?}"),
        ("underleftarrow", "Mũi tên dưới trái", "\\underleftarrow{#?}"),
        ("underrightarrow", "Mũi tên dưới phải", "\\underrightarrow{#?}"),
        ("underleftrightarrow", "Mũi tên dưới hai chiều", "\\underleftrightarrow{#?}"),
        ("cancel", "Gạch chéo lên", "\\cancel{#?}"),
        ("bcancel", "Gạch chéo xuống", "\\bcancel{#?}"),
        ("xcancel", "Gạch chéo chữ X", "\\xcancel{#?}"),
        ("underhat", "Mũ dưới", "\\underhat{#?}"),
        ("undertilde", "Sóng dưới", "\\undertilde{#?}"),
        ("undercheck", "Háček dưới", "\\undercheck{#?}"),
        ("underbreve", "Breve dưới", "\\underbreve{#?}"),
        ("underring", "Vòng tròn dưới", "\\underring{#?}"),
        ("overleftharpoon", "Móc trên trái", "\\overleftharpoon{#?}"),
        ("overrightharpoon", "Móc trên phải", "\\overrightharpoon{#?}"),
        ("underleftharpoon", "Móc dưới trái", "\\underleftharpoon{#?}"),
        ("underrightharpoon", "Móc dưới phải", "\\underrightharpoon{#?}"),
        ("overaccent", "Dấu acute trên", "\\overaccent{ˊ}{#?}"),
        ("underaccent", "Dấu acute dưới", "\\underaccent{ˊ}{#?}"),
        ("overline", "Gạch trên", "\\overline{#?}"),
        ("underline", "Gạch dưới", "\\underline{#?}"),
        ("doubleoverline", "Gạch kép trên", "\\doubleoverline{#?}"),
        ("doubleunderline", "Gạch kép dưới", "\\doubleunderline{#?}"),
        ("overrightarrow", "Mũi tên trên phải", "\\overrightarrow{#?}"),
        ("lim", "Giới hạn", "\\lim"),
        ("sin", "Sin", "\\sin"),
        ("cos", "Cos", "\\cos"),
        ("tan", "Tan", "\\tan"),
        ("cot", "Cotang", "\\cot"),
        ("sec", "Secant", "\\sec"),
        ("csc", "Cosecant", "\\csc"),
        ("arcsin", "Arcsin", "\\arcsin"),
        ("arccos", "Arccos", "\\arccos"),
        ("arctan", "Arctan", "\\arctan"),
        ("sinh", "Sinh", "\\sinh"),
        ("cosh", "Cosh", "\\cosh"),
        ("tanh", "Tanh", "\\tanh"),
        ("log", "Logarit", "\\log"),
        ("ln", "Logarit tự nhiên", "\\ln"),
        ("exp", "Hàm mũ", "\\exp"),
        ("min", "Nhỏ nhất", "\\min"),
        ("max", "Lớn nhất", "\\max"),
        ("inf", "Cận dưới", "\\inf"),
        ("sup", "Cận trên", "\\sup"),
        ("det", "Định thức", "\\det"),
        ("to", "Mũi tên phải", "\\rightarrow"),
        ("iff", "Khi và chỉ khi", "\\Leftrightarrow"),
        ("implies", "Suy ra", "\\Rightarrow"),
        ("leq", "Nhỏ hơn hoặc bằng", "\\le"),
        ("geq", "Lớn hơn hoặc bằng", "\\ge"),
        ("ne", "Khác", "\\neq"),
        ("dots", "Ba chấm", "\\ldots"),
        ("bar", "Gạch trên", "\\overline{#?}"),
        ("pmatrix", "Ma trận ngoặc tròn", "\\begin{pmatrix} #? & #? \\\\ #? & #? \\end{pmatrix}"),
        ("bmatrix", "Ma trận ngoặc vuông", "\\begin{bmatrix} #? & #? \\\\ #? & #? \\end{bmatrix}"),
        ("cases", "Hệ phương trình", "\\begin{cases} #? & #? \\\\ #? & #? \\end{cases}"),
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
