//! MTEF 5.0 template selector, variation, alignment, and embellishment constants.

// ── Additional size record constants (MTEF 5) ───────────────────────────────

/// `SYM` size record (`0x0D`): symbol size (used for big-operator characters).
pub const SYM: &[u8] = b"\x0d";
/// `SUBSYM` size record (`0x0E`): sub-symbol size.
pub const SUBSYM: &[u8] = b"\x0e";

// ── Template selector constants (0..=37) ────────────────────────────────────

/// `tmANGLE` (`0`): angle brackets `< ... >`.
pub const TM_ANGLE: u8 = 0;
/// `tmPAREN` (`1`): parentheses `( ... )`.
pub const TM_PAREN: u8 = 1;
/// `tmBRACE` (`2`): braces `{ ... }`.
pub const TM_BRACE: u8 = 2;
/// `tmBRACK` (`3`): square brackets `[ ... ]`.
pub const TM_BRACK: u8 = 3;
/// `tmBAR` (`4`): vertical bars `| ... |`.
pub const TM_BAR: u8 = 4;
/// `tmDBAR` (`5`): double vertical bars `|| ... ||`.
pub const TM_DBAR: u8 = 5;
/// `tmFLOOR` (`6`): floor brackets.
pub const TM_FLOOR: u8 = 6;
/// `tmCEILING` (`7`): ceiling brackets.
pub const TM_CEILING: u8 = 7;
/// `tmOBRACK` (`8`): open (white/tortoise-shell) brackets.
pub const TM_OBRACK: u8 = 8;
/// `tmINTERVAL` (`9`): unmatched interval fences.
pub const TM_INTERVAL: u8 = 9;
/// `tmROOT` (`10`): square root / nth root radical.
pub const TM_ROOT: u8 = 10;
/// `tmFRACT` (`11`): fraction.
pub const TM_FRACT: u8 = 11;
/// `tmUBAR` (`12`): underbar.
pub const TM_UBAR: u8 = 12;
/// `tmOBAR` (`13`): overbar.
pub const TM_OBAR: u8 = 13;
/// `tmARROW` (`14`): extensible arrow with text above/below.
pub const TM_ARROW: u8 = 14;
/// `tmINTEG` (`15`): integral.
pub const TM_INTEG: u8 = 15;
/// `tmSUM` (`16`): summation.
pub const TM_SUM: u8 = 16;
/// `tmPROD` (`17`): product.
pub const TM_PROD: u8 = 17;
/// `tmCOPROD` (`18`): coproduct.
pub const TM_COPROD: u8 = 18;
/// `tmUNION` (`19`): union.
pub const TM_UNION: u8 = 19;
/// `tmINTER` (`20`): intersection.
pub const TM_INTER: u8 = 20;
/// `tmINTOP` (`21`): integral-style big operator.
pub const TM_INTOP: u8 = 21;
/// `tmSUMOP` (`22`): summation-style big operator.
pub const TM_SUMOP: u8 = 22;
/// `tmLIM` (`23`): limits (underscript / overscript / under-overscript).
pub const TM_LIM: u8 = 23;
/// `tmHBRACE` (`24`): horizontal brace (overbrace / underbrace).
pub const TM_HBRACE: u8 = 24;
/// `tmHBRACK` (`25`): horizontal bracket (overbracket / underbracket).
pub const TM_HBRACK: u8 = 25;
/// `tmLDIV` (`26`): long division.
pub const TM_LDIV: u8 = 26;
/// `tmSUB` (`27`): subscript.
pub const TM_SUB: u8 = 27;
/// `tmSUP` (`28`): superscript.
pub const TM_SUP: u8 = 28;
/// `tmSUBSUP` (`29`): subscript and superscript.
pub const TM_SUBSUP: u8 = 29;
/// `tmDIRAC` (`30`): Dirac bra-ket structure.
pub const TM_DIRAC: u8 = 30;
/// `tmVEC` (`31`): vector / harpoon arrow over or under a slot.
pub const TM_VEC: u8 = 31;
/// `tmTILDE` (`32`): tilde over a slot.
pub const TM_TILDE: u8 = 32;
/// `tmHAT` (`33`): hat / circumflex over a slot.
pub const TM_HAT: u8 = 33;
/// `tmARC` (`34`): arc / frown over a slot.
pub const TM_ARC: u8 = 34;
/// `tmJSTATUS` (`35`): joint-status over a slot.
pub const TM_JSTATUS: u8 = 35;
/// `tmSTRIKE` (`36`): strike-through / cancellation.
pub const TM_STRIKE: u8 = 36;
/// `tmBOX` (`37`): rectangular or rounded box.
pub const TM_BOX: u8 = 37;

// ── Template variation flags ────────────────────────────────────────────────

// Fences (selectors 0..=8)
/// Left fence is present (`tvFENCE_L`).
pub const TV_FENCE_L: u8 = 0x01;
/// Right fence is present (`tvFENCE_R`).
pub const TV_FENCE_R: u8 = 0x02;

// Intervals (selector 9)
/// Left fence is left parenthesis `(` (`tvINTV_LEFT_LP`).
pub const TV_INTV_LEFT_LP: u8 = 0x00;
/// Left fence is right parenthesis `)` (`tvINTV_LEFT_RP`).
pub const TV_INTV_LEFT_RP: u8 = 0x01;
/// Left fence is left bracket `[` (`tvINTV_LEFT_LB`).
pub const TV_INTV_LEFT_LB: u8 = 0x02;
/// Left fence is right bracket `]` (`tvINTV_LEFT_RB`).
pub const TV_INTV_LEFT_RB: u8 = 0x03;
/// Right fence is left parenthesis `(` (`tvINTV_RIGHT_LP`).
pub const TV_INTV_RIGHT_LP: u8 = 0x00;
/// Right fence is right parenthesis `)` (`tvINTV_RIGHT_RP`).
pub const TV_INTV_RIGHT_RP: u8 = 0x10;
/// Right fence is left bracket `[` (`tvINTV_RIGHT_LB`).
pub const TV_INTV_RIGHT_LB: u8 = 0x20;
/// Right fence is right bracket `]` (`tvINTV_RIGHT_RB`).
pub const TV_INTV_RIGHT_RB: u8 = 0x30;

// Radicals (selector 10)
/// Square root (`tvROOT_SQ`).
pub const TV_ROOT_SQ: u8 = 0x00;
/// Nth root (`tvROOT_NTH`).
pub const TV_ROOT_NTH: u8 = 0x01;

// Fractions (selector 11)
/// Subscript-size slots (piece fraction, `tvFR_SMALL`).
pub const TV_FR_SMALL: u8 = 0x01;
/// Diagonal slash fraction (`tvFR_SLASH`).
pub const TV_FR_SLASH: u8 = 0x02;
/// Numerator and denominator on baseline (`tvFR_BASE`).
pub const TV_FR_BASE: u8 = 0x04;

// Underbar / Overbar (selectors 12, 13)
/// Double bar (`tvBAR_DOUBLE`); `0` is a single bar.
pub const TV_BAR_DOUBLE: u8 = 0x01;

// Extensible arrows (selector 14)
/// Single arrow (`tvAR_SINGLE`).
pub const TV_AR_SINGLE: u8 = 0x00;
/// Double arrow (`tvAR_DOUBLE`, e.g. `⇄`).
pub const TV_AR_DOUBLE: u8 = 0x01;
/// Harpoon (`tvAR_HARPOON`, e.g. `⇌`).
pub const TV_AR_HARPOON: u8 = 0x02;
/// Top slot is present (`tvAR_TOP`).
pub const TV_AR_TOP: u8 = 0x04;
/// Bottom slot is present (`tvAR_BOTTOM`).
pub const TV_AR_BOTTOM: u8 = 0x08;
/// Both top and bottom slots present (`tvAR_TOPBOTTOM`).
pub const TV_AR_TOPBOTTOM: u8 = TV_AR_TOP | TV_AR_BOTTOM;
/// Arrow points left (`tvAR_LEFT`; with `TV_AR_SINGLE`).
pub const TV_AR_LEFT: u8 = 0x10;
/// Arrow points right (`tvAR_RIGHT`; with `TV_AR_SINGLE`).
pub const TV_AR_RIGHT: u8 = 0x20;
/// Large over small (`tvAR_LOS`; with `TV_AR_DOUBLE` or `TV_AR_HARPOON`).
pub const TV_AR_LOS: u8 = 0x40;
/// Small over large (`tvAR_SOL`; with `TV_AR_DOUBLE` or `TV_AR_HARPOON`).
pub const TV_AR_SOL: u8 = 0x80;

// Integrals (selector 15)
/// Single integral sign (`tvINT_1`).
pub const TV_INT_1: u16 = 0x0001;
/// Double integral sign (`tvINT_2`).
pub const TV_INT_2: u16 = 0x0002;
/// Triple integral sign (`tvINT_3`).
pub const TV_INT_3: u16 = 0x0003;
/// Contour loop integral (`tvINT_LOOP`, no direction arrows).
pub const TV_INT_LOOP: u16 = 0x0004;
/// Clockwise loop integral (`tvINT_CW_LOOP`).
pub const TV_INT_CW_LOOP: u16 = 0x0008;
/// Counter-clockwise loop integral (`tvINT_CCW_LOOP`).
pub const TV_INT_CCW_LOOP: u16 = 0x000C;
/// Expandable integral sign (`tvINT_EXPAND`).
pub const TV_INT_EXPAND: u16 = 0x0100;

// Big operators & Limits (selectors 15..=23)
/// Lower limit is present (`tvBO_LOWER`).
pub const TV_BO_LOWER: u16 = 0x0010;
/// Upper limit is present (`tvBO_UPPER`).
pub const TV_BO_UPPER: u16 = 0x0020;
/// Summation-style limits centered above/below (`tvBO_SUM`); if clear, integral-style.
pub const TV_BO_SUM: u16 = 0x0040;

// Limits underbar variants (selector 23)
/// Single underbar on limit (`tvSUBAR`).
pub const TV_SUBAR: u8 = 0x00;
/// Double underbar on limit (`tvDUBAR`).
pub const TV_DUBAR: u8 = 0x01;

// Horizontal braces / brackets (selectors 24, 25)
/// Slot is on top (`tvHB_TOP`); otherwise on bottom.
pub const TV_HB_TOP: u8 = 0x01;

// Long division (selector 26)
/// Upper quotient slot is present (`tvLD_UPPER`).
pub const TV_LD_UPPER: u8 = 0x01;

// Subscript / Superscript (selectors 27, 28, 29)
/// Script precedes scripted item (`tvSU_PRECEDES`, e.g. prescripts).
pub const TV_SU_PRECEDES: u8 = 0x01;

// Dirac bra-ket (selector 30)
/// Left part (bra) is present (`tvDI_LEFT`).
pub const TV_DI_LEFT: u8 = 0x01;
/// Right part (ket) is present (`tvDI_RIGHT`).
pub const TV_DI_RIGHT: u8 = 0x02;

// Vector / harpoon arrow (selector 31)
/// Arrow points left (`tvVE_LEFT`).
pub const TV_VE_LEFT: u8 = 0x01;
/// Arrow points right (`tvVE_RIGHT`).
pub const TV_VE_RIGHT: u8 = 0x02;
/// Arrow is under slot (`tvVE_UNDER`), else over slot.
pub const TV_VE_UNDER: u8 = 0x04;
/// Harpoon (single barb, `tvVE_HARPOON`), else regular arrow.
pub const TV_VE_HARPOON: u8 = 0x08;

// Strike-through (selector 36)
/// Horizontal strike (`tvST_HORIZ`).
pub const TV_ST_HORIZ: u8 = 0x01;
/// Bottom-left to upper-right strike (`tvST_UP`).
pub const TV_ST_UP: u8 = 0x02;
/// Top-left to bottom-right strike (`tvST_DOWN`).
pub const TV_ST_DOWN: u8 = 0x04;

// Box (selector 37)
/// Rounded corners (`tvBX_ROUND`).
pub const TV_BX_ROUND: u8 = 0x01;
/// Left border present (`tvBX_LEFT`).
pub const TV_BX_LEFT: u8 = 0x02;
/// Right border present (`tvBX_RIGHT`).
pub const TV_BX_RIGHT: u8 = 0x04;
/// Top border present (`tvBX_TOP`).
pub const TV_BX_TOP: u8 = 0x08;
/// Bottom border present (`tvBX_BOTTOM`).
pub const TV_BX_BOTTOM: u8 = 0x10;

// ── Pile / Matrix alignment constants ───────────────────────────────────────

/// Horizontal alignment: left (`1`).
pub const HALIGN_LEFT: u8 = 1;
/// Horizontal alignment: center (`2`).
pub const HALIGN_CENTER: u8 = 2;
/// Horizontal alignment: right (`3`).
pub const HALIGN_RIGHT: u8 = 3;
/// Horizontal alignment: relational operator (`4`).
pub const HALIGN_AL: u8 = 4;
/// Horizontal alignment: decimal point (`5`).
pub const HALIGN_DEC: u8 = 5;

/// Vertical alignment: top baseline (`0`).
pub const VALIGN_TOP_BASELINE: u8 = 0;
/// Vertical alignment: center baseline (`1`).
pub const VALIGN_CENTER_BASELINE: u8 = 1;
/// Vertical alignment: bottom baseline (`2`).
pub const VALIGN_BOTTOM_BASELINE: u8 = 2;
/// Vertical alignment: center (`3`).
pub const VALIGN_CENTER: u8 = 3;
/// Vertical alignment: math axis (`4`).
pub const VALIGN_AXIS: u8 = 4;

// ── Character embellishment codes (record type 6) ───────────────────────────

/// Single over-dot embellishment (`2`).
pub const EMB_1DOT: u8 = 2;
/// Double over-dot embellishment (`3`).
pub const EMB_2DOT: u8 = 3;
/// Triple over-dot embellishment (`4`).
pub const EMB_3DOT: u8 = 4;
/// Single prime embellishment (`5`).
pub const EMB_1PRIME: u8 = 5;
/// Double prime embellishment (`6`).
pub const EMB_2PRIME: u8 = 6;
/// Back-prime embellishment (`7`).
pub const EMB_BPRIME: u8 = 7;
/// Over-tilde embellishment (`8`).
pub const EMB_TILDE: u8 = 8;
/// Over-hat / circumflex embellishment (`9`).
pub const EMB_HAT: u8 = 9;
/// Negation slash embellishment (`10`).
pub const EMB_NOT: u8 = 10;
/// Over-right-arrow embellishment (`11`).
pub const EMB_RARROW: u8 = 11;
/// Over-left-arrow embellishment (`12`).
pub const EMB_LARROW: u8 = 12;
/// Over-bidirectional-arrow embellishment (`13`).
pub const EMB_BARROW: u8 = 13;
/// Over-right-harpoon embellishment (`14`).
pub const EMB_R1ARROW: u8 = 14;
/// Over-left-harpoon embellishment (`15`).
pub const EMB_L1ARROW: u8 = 15;
/// Mid-height strike bar embellishment (`16`).
pub const EMB_MBAR: u8 = 16;
/// Overbar embellishment (`17`).
pub const EMB_OBAR: u8 = 17;
/// Triple prime embellishment (`18`).
pub const EMB_3PRIME: u8 = 18;
/// Over-frown arc embellishment (`19`).
pub const EMB_FROWN: u8 = 19;
/// Over-smile arc embellishment (`20`).
pub const EMB_SMILE: u8 = 20;
/// Crossed-out (X) bars embellishment (`21`).
pub const EMB_X_BARS: u8 = 21;
/// Bottom-left to top-right slash bar embellishment (`22`).
pub const EMB_UP_BAR: u8 = 22;
/// Top-left to bottom-right slash bar embellishment (`23`).
pub const EMB_DOWN_BAR: u8 = 23;
/// Quadruple over-dot embellishment (`24`).
pub const EMB_4DOT: u8 = 24;
/// Single under-dot embellishment (`25`).
pub const EMB_U_1DOT: u8 = 25;
/// Double under-dot embellishment (`26`).
pub const EMB_U_2DOT: u8 = 26;
/// Triple under-dot embellishment (`27`).
pub const EMB_U_3DOT: u8 = 27;
/// Quadruple under-dot embellishment (`28`).
pub const EMB_U_4DOT: u8 = 28;
/// Underbar embellishment (`29`).
pub const EMB_U_BAR: u8 = 29;
/// Under-tilde embellishment (`30`).
pub const EMB_U_TILDE: u8 = 30;
/// Under-frown arc embellishment (`31`).
pub const EMB_U_FROWN: u8 = 31;
/// Under-smile arc embellishment (`32`).
pub const EMB_U_SMILE: u8 = 32;
/// Under-right-arrow embellishment (`33`).
pub const EMB_U_RARROW: u8 = 33;
/// Under-left-arrow embellishment (`34`).
pub const EMB_U_LARROW: u8 = 34;
/// Under-bidirectional-arrow embellishment (`35`).
pub const EMB_U_BARROW: u8 = 35;
/// Under-right-harpoon embellishment (`36`).
pub const EMB_U_R1ARROW: u8 = 36;
/// Under-left-harpoon embellishment (`37`).
pub const EMB_U_L1ARROW: u8 = 37;
