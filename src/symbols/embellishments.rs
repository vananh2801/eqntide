//! Embellishments palette: accents drawn over or under a base, and prime marks.

/// Characters accepted as an accent over a base.
pub const ACCENTS: &str = "̀́̆̈̊̌⃖⃡⃐⃑ˊˋ¨⃛⃜ˇ˘˚°⏞⏟⎴⎵⏝⌣←↔⇒⇐⇔⇀↼⇌⇋_═̳̿\u{2312}\u{23DC}\u{2322}\u{20D7}\u{2192}\u{27F6}^\u{0302}\u{02C6}\u{00AF}\u{0304}\u{203E}\u{2015}\u{02C9}~\u{0303}\u{02DC}\u{0307}\u{02D9}\u{00B7}.";

/// `true` when `c` is an accent character.
pub fn is_accent(c: char) -> bool {
    ACCENTS.contains(c)
}

/// Returns true if character is a prime mark or apostrophe.
pub const PRIME_CHARS: &[char] = &['\'', '\u{2032}', '\u{2033}', '\u{2034}', '\u{00B4}'];

pub fn is_prime_char(c: char) -> bool {
    matches!(c, '\'' | '\u{2032}' | '\u{2033}' | '\u{2034}' | '\u{00B4}')
}

/// Returns the order of a derivative prime mark (1 for ', 2 for '', 3 for ''', etc.), or None if not a prime.
pub fn prime_order(text: &str) -> Option<u8> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    match t {
        "'" | "\u{2032}" | "\u{00B4}" => Some(1),
        "''" | "\u{2033}" => Some(2),
        "'''" | "\u{2034}" => Some(3),
        _ => {
            let chars: Vec<char> = t.chars().collect();
            if !chars.is_empty()
                && chars.iter().all(|&c| {
                    c == '\''
                        || c == '\u{2032}'
                        || c == '\u{2033}'
                        || c == '\u{2034}'
                        || c == '\u{00B4}'
                })
            {
                let mut count = 0u8;
                for c in chars {
                    match c {
                        '\'' | '\u{2032}' | '\u{00B4}' => count = count.saturating_add(1),
                        '\u{2033}' => count = count.saturating_add(2),
                        '\u{2034}' => count = count.saturating_add(3),
                        _ => {}
                    }
                }
                match count {
                    1 => Some(1),
                    2 => Some(2),
                    3.. => Some(count.min(3)),
                    0 => None,
                }
            } else {
                None
            }
        }
    }
}
