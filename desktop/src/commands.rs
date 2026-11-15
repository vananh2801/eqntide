//! Local LaTeX command completion for the structural canvas.
use crate::palette;

#[derive(Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    pub label: &'static str,
    pub source: &'static str,
}

#[derive(Default)]
pub struct Input {
    pub name: String,
    pub selected: usize,
}

const COMMANDS: &[(&str, &str, &str)] = &[
    ("frac", "Phân số", r"\frac{#?}{#?}"),
    ("dfrac", "Phân số", r"\dfrac{#?}{#?}"),
    ("tfrac", "Phân số nhỏ", r"\tfrac{#?}{#?}"),
    ("sqrt", "Căn bậc hai", r"\sqrt{#?}"),
    ("vec", "Vectơ", r"\vec{#?}"),
    ("hat", "Dấu mũ", r"\hat{#?}"),
    ("bar", "Gạch trên", r"\bar{#?}"),
    ("overline", "Gạch trên", r"\overline{#?}"),
    ("underline", "Gạch dưới", r"\underline{#?}"),
    ("dot", "Một chấm", r"\dot{#?}"),
    ("ddot", "Hai chấm", r"\ddot{#?}"),
    ("lim", "Giới hạn", r"\lim"),
    ("sum", "Tổng", r"\sum"),
    ("prod", "Tích", r"\prod"),
    ("int", "Tích phân", r"\int"),
    ("iint", "Tích phân kép", r"\iint"),
    ("iiint", "Tích phân ba", r"\iiint"),
    ("oint", "Tích phân vòng", r"\oint"),
    ("infty", "Vô cực", r"\infty"),
    ("rightarrow", "Mũi tên phải", r"\rightarrow"),
    ("leftarrow", "Mũi tên trái", r"\leftarrow"),
    ("leftrightarrow", "Mũi tên hai chiều", r"\leftrightarrow"),
    ("leq", "Nhỏ hơn hoặc bằng", r"\leq"),
    ("geq", "Lớn hơn hoặc bằng", r"\geq"),
    ("neq", "Khác", r"\neq"),
    ("sin", "Sin", r"\sin"),
    ("cos", "Cos", r"\cos"),
    ("tan", "Tan", r"\tan"),
    ("log", "Logarit", r"\log"),
    ("ln", "Logarit tự nhiên", r"\ln"),
    ("exp", "Hàm mũ", r"\exp"),
    ("min", "Giá trị nhỏ nhất", r"\min"),
    ("max", "Giá trị lớn nhất", r"\max"),
    (
        "pmatrix",
        "Ma trận ngoặc tròn",
        r"\begin{pmatrix} #? & #? \\ #? & #? \end{pmatrix}",
    ),
    (
        "bmatrix",
        "Ma trận ngoặc vuông",
        r"\begin{bmatrix} #? & #? \\ #? & #? \end{bmatrix}",
    ),
    (
        "cases",
        "Hệ phương trình",
        r"\begin{cases} #? & #? \\ #? & #? \end{cases}",
    ),
];

pub fn suggestions(prefix: &str) -> Vec<Command> {
    let mut commands: Vec<_> = COMMANDS
        .iter()
        .map(|&(name, label, source)| Command {
            name,
            label,
            source,
        })
        .chain(
            palette::PALETTE
                .iter()
                .filter_map(|&(group, label, source)| {
                    let name = source.strip_prefix('\\')?;
                    (group == "Symbols" && name.chars().all(|c| c.is_ascii_alphabetic())).then_some(
                        Command {
                            name,
                            label,
                            source,
                        },
                    )
                }),
        )
        .filter(|c| c.name.starts_with(prefix))
        .collect();
    commands.sort_by_key(|c| (c.name != prefix, c.name));
    commands.dedup_by_key(|c| c.name);
    commands.truncate(8);
    commands
}

pub fn exact(name: &str) -> Option<Command> {
    suggestions(name).into_iter().find(|c| c.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_completion_is_an_insertable_template() {
        for &(name, _, source) in COMMANDS {
            assert!(palette::template(source).is_ok(), "{name}");
        }
        for prefix in ["lim", "infty", "rightarrow", "alpha", "frac", "dfrac"] {
            assert_eq!(suggestions(prefix)[0].name, prefix);
            assert!(exact(prefix).is_some());
        }
        assert!(suggestions("nosuchcommand").is_empty());
    }
}
