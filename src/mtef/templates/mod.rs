//! MTEF 5.0 template constructors, selector constants, variation flags, and
//! composite records (`PILE`, `MATRIX`, `EMBELL`).

mod basic;
mod composite;
mod consts;
mod slots;

#[cfg(test)]
mod tests;

pub use basic::{
    base_slash_fraction, fraction, fraction_variant, nroot_radical, presubscript,
    presubsuperscript, presuperscript, script_template, slash_fraction, small_fraction,
    sqrt_radical, subscript, subsuperscript, superscript,
};
pub use composite::{
    angle_brackets, bar_variant, big_operator, boxed, braces, brackets, ceiling_brackets,
    coproduct, dirac, double_overbar, double_underbar, double_vertical_bars, embell_record,
    embellished_char, extensible_arrow, fence, floor_brackets, harpoon_arrow, hat_box_template,
    horizontal_fence, int_op, integral, intersection_op, interval, joint_status, limits,
    long_division, matrix, one_slot_template, open_brackets, over_arc, over_hat, over_tilde,
    overbar, overbrace, overbracket, overscript, parens, pile, product, strike_through, sum_op,
    summation, under_harpoon_arrow, under_overscript, under_vector_arrow, underbar, underbrace,
    underbracket, underscript, union_op, vector_arrow, vertical_bars,
};
pub use consts::*;
