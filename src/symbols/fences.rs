//! Fence symbols (parentheses, brackets, braces, bars).

/// Opening fences.
pub const OPEN: &str = "([{⟨⌊⌈";

/// Closing fences.
pub const CLOSE: &str = ")]}⟩⌋⌉";

/// Vertical bars.
pub const BARS: &str = "|‖";

/// `true` for an opening fence.
pub fn is_open(c: char) -> bool {
    OPEN.contains(c)
}

/// `true` for a closing fence.
pub fn is_close(c: char) -> bool {
    CLOSE.contains(c)
}

/// `true` for a vertical bar.
pub fn is_bar(c: char) -> bool {
    BARS.contains(c)
}

/// The opening fence matching a closing one (other characters map to themselves).
pub fn open_for(c: char) -> char {
    match c {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        '⟩' => '⟨',
        '⌋' => '⌊',
        '⌉' => '⌈',
        o => o,
    }
}
