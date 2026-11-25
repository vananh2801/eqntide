//! MTEF binary record type tags and option flag constants (v3 and v5).

pub(super) const REC_END: u8 = 0;
pub(super) const REC_LINE: u8 = 1;
pub(super) const REC_CHAR: u8 = 2;
pub(super) const REC_TMPL: u8 = 3;
pub(super) const REC_PILE: u8 = 4;
pub(super) const REC_MATRIX: u8 = 5;
pub(super) const REC_EMBELL: u8 = 6;
pub(super) const REC_RULER: u8 = 7;
pub(super) const REC_FONT_STYLE_DEF: u8 = 8; // v5: FONT_STYLE_DEF, v3: FONT
pub(super) const REC_SIZE: u8 = 9;
pub(super) const REC_FULL: u8 = 10;
pub(super) const REC_SUB: u8 = 11;
pub(super) const REC_SUB2: u8 = 12;
pub(super) const REC_SYM: u8 = 13;
pub(super) const REC_SUBSYM: u8 = 14;
pub(super) const REC_COLOR: u8 = 15;
pub(super) const REC_COLOR_DEF: u8 = 16;
pub(super) const REC_FONT_DEF: u8 = 17;
pub(super) const REC_EQN_PREFS: u8 = 18;
pub(super) const REC_ENCODING_DEF: u8 = 19;
pub(super) const REC_MT_COMMENT: u8 = 102;

pub(super) const OPT_NUDGE: u8 = 0x08;
pub(super) const OPT_CHAR_EMBELL: u8 = 0x01;
pub(super) const OPT_CHAR_ENC_8: u8 = 0x04;
pub(super) const OPT_CHAR_ENC_16: u8 = 0x10;
pub(super) const OPT_CHAR_NO_MT: u8 = 0x20;
pub(super) const OPT_LINE_NULL: u8 = 0x01;
pub(super) const OPT_LP_RULER: u8 = 0x02;
pub(super) const OPT_LINE_LSPACE: u8 = 0x04;
pub(super) const OPT_COLOR_CMYK: u8 = 0x01;
pub(super) const OPT_COLOR_NAME: u8 = 0x04;
