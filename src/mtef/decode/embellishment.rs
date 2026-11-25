//! MathML wrappers for MTEF character embellishments (`REC_EMBELL`) and enclosures.

use crate::mathml::MathNode;

use super::characters::append_prime;

/// Wrap `node` with each MTEF embellishment code attached to a `REC_CHAR` record.
pub(super) fn wrap_embellishments(mut node: MathNode, embells: &[u8]) -> MathNode {
    for &code in embells {
        node = match code {
            // Over dots and accents
            2 => mover_accent(node, "\u{02D9}"),
            3 => mover_accent(node, "\u{00A8}"),
            4 => mover_accent(node, "\u{20DB}"),
            24 => mover_accent(node, "\u{20DC}"),
            8 => mover_accent(node, "\u{02DC}"),
            9 => mover_accent(node, "\u{02C6}"),
            11 => mover_accent(node, "\u{2192}"),
            12 => mover_accent(node, "\u{2190}"),
            13 => mover_accent(node, "\u{2194}"),
            14 => mover_accent(node, "\u{21C0}"),
            15 => mover_accent(node, "\u{21BC}"),
            17 => mover_accent(node, "\u{00AF}"),
            19 => mover_accent(node, "\u{2322}"),
            20 => mover_accent(node, "\u{2323}"),

            // Primes and back-prime
            5 => append_prime(node, "'"),
            6 => append_prime(node, "''"),
            18 => append_prime(node, "'''"),
            7 => MathNode::with_children(
                "mmultiscripts",
                vec![
                    node,
                    MathNode::new("mprescripts"),
                    MathNode::new("none"),
                    MathNode::leaf("mo", "\u{2035}"),
                ],
            ),

            // Diagonal and horizontal strikes
            10 | 22 => menclose(node, "updiagonalstrike"),
            16 => menclose(node, "horizontalstrike"),
            21 => menclose(node, "updiagonalstrike downdiagonalstrike"),
            23 => menclose(node, "downdiagonalstrike"),

            // Under dots, accents, and arrows
            25 => munder(node, "\u{02D9}", true),
            26 => munder(node, "\u{00A8}", true),
            27 => munder(node, "\u{20DB}", true),
            28 => munder(node, "\u{20DC}", true),
            29 => munder(node, "_", true),
            30 => munder(node, "\u{02DC}", true),
            31 => munder(node, "\u{2322}", true),
            32 => munder(node, "\u{2323}", true),
            33 => munder(node, "\u{2192}", true),
            34 => munder(node, "\u{2190}", true),
            35 => munder(node, "\u{2194}", true),
            36 => munder(node, "\u{21C1}", true),
            37 => munder(node, "\u{21BD}", true),
            _ => node,
        };
    }
    node
}

/// Wrap `base` in `<menclose notation="...">`.
pub(super) fn menclose(base: MathNode, notation: &str) -> MathNode {
    let mut node = MathNode::with_children("menclose", vec![base]);
    node.set_attr("notation", notation);
    node
}

fn mover_accent(base: MathNode, mo_str: &str) -> MathNode {
    let mut node = MathNode::with_children("mover", vec![base, MathNode::leaf("mo", mo_str)]);
    node.set_attr("accent", "true");
    node
}

fn munder(base: MathNode, mo_str: &str, accentunder: bool) -> MathNode {
    let mut node = MathNode::with_children("munder", vec![base, MathNode::leaf("mo", mo_str)]);
    if accentunder {
        node.set_attr("accentunder", "true");
    }
    node
}
