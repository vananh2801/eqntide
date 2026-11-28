//! Keyboard shortcut chord definitions, defaults, validation, and conflict resolution.

use crate::palette;
use eframe::egui;
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

/// Read-only display entry for the fixed System & Navigation shortcuts group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SystemShortcut {
    /// Human-readable action description shown in Settings.
    pub(crate) label: &'static str,
    /// Display summary of the fixed key binding(s).
    pub(crate) display: &'static str,
}

/// Fixed system and navigation shortcuts displayed as read-only in Settings.
pub(crate) const SYSTEM_SHORTCUTS: &[SystemShortcut] = &[
    SystemShortcut {
        label: "Save equation",
        display: "Ctrl+S",
    },
    SystemShortcut {
        label: "Undo",
        display: "Ctrl+Z",
    },
    SystemShortcut {
        label: "Redo",
        display: "Ctrl+Y / Ctrl+Shift+Z",
    },
    SystemShortcut {
        label: "Select all",
        display: "Ctrl+A",
    },
    SystemShortcut {
        label: "Copy",
        display: "Ctrl+C",
    },
    SystemShortcut {
        label: "Cut",
        display: "Ctrl+X",
    },
    SystemShortcut {
        label: "Paste",
        display: "Ctrl+V",
    },
    SystemShortcut {
        label: "Next placeholder",
        display: "Tab",
    },
    SystemShortcut {
        label: "Previous placeholder",
        display: "Shift+Tab",
    },
    SystemShortcut {
        label: "Close popup / Exit",
        display: "Esc",
    },
];

/// Canonical default palette shortcuts mapped by exact `PALETTE` template `source`.
pub(crate) const DEFAULT_PALETTE_SHORTCUTS: &[(&str, ShortcutChord)] = &[
    (r"\frac{#?}{#?}", ShortcutChord::cmd(ShortcutKey::F)),
    (r"\sqrt{#?}", ShortcutChord::cmd(ShortcutKey::R)),
    (r"\sqrt[#?]{#?}", ShortcutChord::cmd_shift(ShortcutKey::R)),
    (r"^{#?}", ShortcutChord::cmd(ShortcutKey::H)),
    (r"_{#?}", ShortcutChord::cmd(ShortcutKey::L)),
    (r"_{#?}^{#?}", ShortcutChord::cmd(ShortcutKey::J)),
    (
        r"\lim_{#? \to #?}",
        ShortcutChord::cmd_shift(ShortcutKey::L),
    ),
    (r"\int", ShortcutChord::cmd(ShortcutKey::I)),
    (
        r"\int\limits_{#?}^{#?} #?",
        ShortcutChord::cmd_shift(ShortcutKey::I),
    ),
    (
        r"\sum\limits_{#?}^{#?} #?",
        ShortcutChord::cmd_shift(ShortcutKey::S),
    ),
    (
        r"\prod\limits_{#?}^{#?} #?",
        ShortcutChord::cmd_shift(ShortcutKey::P),
    ),
    (
        r"\begin{pmatrix} #? & #? \\ #? & #? \end{pmatrix}",
        ShortcutChord::cmd_shift(ShortcutKey::M),
    ),
    (
        r"\begin{cases} #? & #? \\ #? & #? \end{cases}",
        ShortcutChord::cmd_shift(ShortcutKey::C),
    ),
    (r"\left( #? \right)", ShortcutChord::cmd(ShortcutKey::Num9)),
    (
        r"\left[ #? \right]",
        ShortcutChord::cmd(ShortcutKey::OpenBracket),
    ),
];

const RESERVED_SYSTEM_CHORDS: &[(&str, ShortcutChord)] = &[
    ("Save equation", ShortcutChord::cmd(ShortcutKey::S)),
    ("Undo", ShortcutChord::cmd(ShortcutKey::Z)),
    ("Redo", ShortcutChord::cmd(ShortcutKey::Y)),
    ("Redo", ShortcutChord::cmd_shift(ShortcutKey::Z)),
    ("Select all", ShortcutChord::cmd(ShortcutKey::A)),
    ("Copy", ShortcutChord::cmd(ShortcutKey::C)),
    ("Cut", ShortcutChord::cmd(ShortcutKey::X)),
    ("Paste", ShortcutChord::cmd(ShortcutKey::V)),
];

/// Supported primary keys for customizable palette shortcuts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum ShortcutKey {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    OpenBracket,
    CloseBracket,
    Comma,
    Period,
    Semicolon,
    Minus,
    Equals,
    Slash,
    Backslash,
    Backtick,
    Quote,
    Plus,
}

impl ShortcutKey {
    /// Converts an `egui::Key` into a supported [`ShortcutKey`], or `None` if unsupported.
    pub(crate) const fn from_egui(key: egui::Key) -> Option<Self> {
        Some(match key {
            egui::Key::A => Self::A,
            egui::Key::B => Self::B,
            egui::Key::C => Self::C,
            egui::Key::D => Self::D,
            egui::Key::E => Self::E,
            egui::Key::F => Self::F,
            egui::Key::G => Self::G,
            egui::Key::H => Self::H,
            egui::Key::I => Self::I,
            egui::Key::J => Self::J,
            egui::Key::K => Self::K,
            egui::Key::L => Self::L,
            egui::Key::M => Self::M,
            egui::Key::N => Self::N,
            egui::Key::O => Self::O,
            egui::Key::P => Self::P,
            egui::Key::Q => Self::Q,
            egui::Key::R => Self::R,
            egui::Key::S => Self::S,
            egui::Key::T => Self::T,
            egui::Key::U => Self::U,
            egui::Key::V => Self::V,
            egui::Key::W => Self::W,
            egui::Key::X => Self::X,
            egui::Key::Y => Self::Y,
            egui::Key::Z => Self::Z,
            egui::Key::Num0 => Self::Num0,
            egui::Key::Num1 => Self::Num1,
            egui::Key::Num2 => Self::Num2,
            egui::Key::Num3 => Self::Num3,
            egui::Key::Num4 => Self::Num4,
            egui::Key::Num5 => Self::Num5,
            egui::Key::Num6 => Self::Num6,
            egui::Key::Num7 => Self::Num7,
            egui::Key::Num8 => Self::Num8,
            egui::Key::Num9 => Self::Num9,
            egui::Key::F1 => Self::F1,
            egui::Key::F2 => Self::F2,
            egui::Key::F3 => Self::F3,
            egui::Key::F4 => Self::F4,
            egui::Key::F5 => Self::F5,
            egui::Key::F6 => Self::F6,
            egui::Key::F7 => Self::F7,
            egui::Key::F8 => Self::F8,
            egui::Key::F9 => Self::F9,
            egui::Key::F10 => Self::F10,
            egui::Key::F11 => Self::F11,
            egui::Key::F12 => Self::F12,
            egui::Key::OpenBracket => Self::OpenBracket,
            egui::Key::CloseBracket => Self::CloseBracket,
            egui::Key::Comma => Self::Comma,
            egui::Key::Period => Self::Period,
            egui::Key::Semicolon => Self::Semicolon,
            egui::Key::Minus => Self::Minus,
            egui::Key::Equals => Self::Equals,
            egui::Key::Slash => Self::Slash,
            egui::Key::Backslash => Self::Backslash,
            egui::Key::Backtick => Self::Backtick,
            egui::Key::Quote => Self::Quote,
            egui::Key::Plus => Self::Plus,
            _ => return None,
        })
    }

    /// Returns `true` when this key is a function key (`F1`..=`F12`).
    pub(crate) const fn is_function_key(self) -> bool {
        matches!(
            self,
            Self::F1
                | Self::F2
                | Self::F3
                | Self::F4
                | Self::F5
                | Self::F6
                | Self::F7
                | Self::F8
                | Self::F9
                | Self::F10
                | Self::F11
                | Self::F12
        )
    }

    /// Returns the canonical token representation used for formatting and JSON storage.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::H => "H",
            Self::I => "I",
            Self::J => "J",
            Self::K => "K",
            Self::L => "L",
            Self::M => "M",
            Self::N => "N",
            Self::O => "O",
            Self::P => "P",
            Self::Q => "Q",
            Self::R => "R",
            Self::S => "S",
            Self::T => "T",
            Self::U => "U",
            Self::V => "V",
            Self::W => "W",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::Num0 => "0",
            Self::Num1 => "1",
            Self::Num2 => "2",
            Self::Num3 => "3",
            Self::Num4 => "4",
            Self::Num5 => "5",
            Self::Num6 => "6",
            Self::Num7 => "7",
            Self::Num8 => "8",
            Self::Num9 => "9",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::OpenBracket => "[",
            Self::CloseBracket => "]",
            Self::Comma => ",",
            Self::Period => ".",
            Self::Semicolon => ";",
            Self::Minus => "-",
            Self::Equals => "=",
            Self::Slash => "/",
            Self::Backslash => "\\",
            Self::Backtick => "`",
            Self::Quote => "'",
            Self::Plus => "Plus",
        }
    }

    /// Parses a canonical key token (case-insensitive for ASCII words/letters).
    pub(crate) fn parse(token: &str) -> Option<Self> {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return None;
        }
        match trimmed {
            "[" => return Some(Self::OpenBracket),
            "]" => return Some(Self::CloseBracket),
            "," => return Some(Self::Comma),
            "." => return Some(Self::Period),
            ";" => return Some(Self::Semicolon),
            "-" => return Some(Self::Minus),
            "=" => return Some(Self::Equals),
            "/" => return Some(Self::Slash),
            "\\" => return Some(Self::Backslash),
            "`" => return Some(Self::Backtick),
            "'" => return Some(Self::Quote),
            _ => {}
        }
        let upper = trimmed.to_ascii_uppercase();
        Some(match upper.as_str() {
            "A" => Self::A,
            "B" => Self::B,
            "C" => Self::C,
            "D" => Self::D,
            "E" => Self::E,
            "F" => Self::F,
            "G" => Self::G,
            "H" => Self::H,
            "I" => Self::I,
            "J" => Self::J,
            "K" => Self::K,
            "L" => Self::L,
            "M" => Self::M,
            "N" => Self::N,
            "O" => Self::O,
            "P" => Self::P,
            "Q" => Self::Q,
            "R" => Self::R,
            "S" => Self::S,
            "T" => Self::T,
            "U" => Self::U,
            "V" => Self::V,
            "W" => Self::W,
            "X" => Self::X,
            "Y" => Self::Y,
            "Z" => Self::Z,
            "0" | "NUM0" => Self::Num0,
            "1" | "NUM1" => Self::Num1,
            "2" | "NUM2" => Self::Num2,
            "3" | "NUM3" => Self::Num3,
            "4" | "NUM4" => Self::Num4,
            "5" | "NUM5" => Self::Num5,
            "6" | "NUM6" => Self::Num6,
            "7" | "NUM7" => Self::Num7,
            "8" | "NUM8" => Self::Num8,
            "9" | "NUM9" => Self::Num9,
            "F1" => Self::F1,
            "F2" => Self::F2,
            "F3" => Self::F3,
            "F4" => Self::F4,
            "F5" => Self::F5,
            "F6" => Self::F6,
            "F7" => Self::F7,
            "F8" => Self::F8,
            "F9" => Self::F9,
            "F10" => Self::F10,
            "F11" => Self::F11,
            "F12" => Self::F12,
            "OPENBRACKET" => Self::OpenBracket,
            "CLOSEBRACKET" => Self::CloseBracket,
            "COMMA" => Self::Comma,
            "PERIOD" => Self::Period,
            "SEMICOLON" => Self::Semicolon,
            "MINUS" => Self::Minus,
            "EQUALS" => Self::Equals,
            "SLASH" => Self::Slash,
            "BACKSLASH" => Self::Backslash,
            "BACKTICK" => Self::Backtick,
            "QUOTE" => Self::Quote,
            "PLUS" => Self::Plus,
            _ => return None,
        })
    }
}

/// Normalized keyboard shortcut chord combining modifiers and a supported [`ShortcutKey`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct ShortcutChord {
    /// Primary platform command modifier (`Ctrl` on Windows/Linux, `Cmd` on macOS).
    pub(crate) command: bool,
    /// Alt / Option modifier.
    pub(crate) alt: bool,
    /// Shift modifier.
    pub(crate) shift: bool,
    /// Primary non-modifier key.
    pub(crate) key: ShortcutKey,
}

impl ShortcutChord {
    /// Constructs a chord with explicit modifier flags and key.
    pub(crate) const fn new(command: bool, alt: bool, shift: bool, key: ShortcutKey) -> Self {
        Self {
            command,
            alt,
            shift,
            key,
        }
    }

    /// Constructs a `Ctrl/Cmd+<key>` chord.
    pub(crate) const fn cmd(key: ShortcutKey) -> Self {
        Self::new(true, false, false, key)
    }

    /// Constructs a `Ctrl/Cmd+Shift+<key>` chord.
    pub(crate) const fn cmd_shift(key: ShortcutKey) -> Self {
        Self::new(true, false, true, key)
    }

    /// Captures a supported chord from an `egui` key press event.
    pub(crate) const fn from_egui_event(
        key: egui::Key,
        modifiers: egui::Modifiers,
    ) -> Option<Self> {
        let Some(mapped_key) = ShortcutKey::from_egui(key) else {
            return None;
        };
        Some(Self {
            command: modifiers.command || modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            key: mapped_key,
        })
    }

    /// Returns `true` when this chord is valid for binding to a customizable palette entry.
    pub(crate) const fn is_valid_palette_chord(&self) -> bool {
        self.command || self.alt || self.key.is_function_key()
    }

    /// Parses a canonical chord string such as `"Ctrl+Shift+R"` or `"F2"`.
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let mut command = false;
        let mut alt = false;
        let mut shift = false;
        let mut parsed_key = None;

        for part in raw.split('+') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return None;
            }
            if trimmed.eq_ignore_ascii_case("ctrl") || trimmed.eq_ignore_ascii_case("cmd") {
                if command {
                    return None;
                }
                command = true;
            } else if trimmed.eq_ignore_ascii_case("alt") {
                if alt {
                    return None;
                }
                alt = true;
            } else if trimmed.eq_ignore_ascii_case("shift") {
                if shift {
                    return None;
                }
                shift = true;
            } else {
                if parsed_key.is_some() {
                    return None;
                }
                parsed_key = Some(ShortcutKey::parse(trimmed)?);
            }
        }

        Some(Self {
            command,
            alt,
            shift,
            key: parsed_key?,
        })
    }

    /// Formats this chord into its canonical string representation (for example `"Ctrl+Shift+R"`).
    pub(crate) fn to_canonical_string(self) -> String {
        let mut parts = Vec::with_capacity(4);
        if self.command {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(self.key.as_str());
        parts.join("+")
    }
}

impl fmt::Display for ShortcutChord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_canonical_string())
    }
}

/// Collision outcome when attempting to assign a chord to a palette entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShortcutConflict {
    /// Collides with a locked system/navigation shortcut (label of the system action).
    System(&'static str),
    /// Collides with another palette entry (canonical `source` of the conflicting entry).
    Palette(&'static str),
}

/// Resolved bidirectional shortcut table for customizable palette entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ShortcutMap {
    by_source: HashMap<&'static str, ShortcutChord>,
    by_chord: HashMap<ShortcutChord, &'static str>,
}

impl Default for ShortcutMap {
    fn default() -> Self {
        Self::default_map()
    }
}

impl ShortcutMap {
    /// Builds the canonical default palette shortcut table.
    pub(crate) fn default_map() -> Self {
        let mut by_source = HashMap::with_capacity(DEFAULT_PALETTE_SHORTCUTS.len());
        let mut by_chord = HashMap::with_capacity(DEFAULT_PALETTE_SHORTCUTS.len());
        for &(raw_source, chord) in DEFAULT_PALETTE_SHORTCUTS {
            if let Some(canonical) = palette::canonical_source(raw_source) {
                by_source.insert(canonical, chord);
                by_chord.insert(chord, canonical);
            }
        }
        Self {
            by_source,
            by_chord,
        }
    }

    /// Resolves the active shortcut table by applying persisted `overrides` onto the default map.
    ///
    /// Unknown sources, invalid chords, system-reserved chords, and duplicate bindings are ignored
    /// safely so each chord maps to at most one valid `PALETTE` entry.
    pub(crate) fn from_overrides(overrides: &BTreeMap<String, Option<String>>) -> Self {
        let mut by_source: HashMap<&'static str, ShortcutChord> = HashMap::new();
        let mut by_chord: HashMap<ShortcutChord, &'static str> = HashMap::new();
        let mut explicitly_cleared = HashMap::new();

        for (raw_source, maybe_chord) in overrides {
            let Some(canonical) = palette::canonical_source(raw_source) else {
                continue;
            };
            match maybe_chord {
                None => {
                    explicitly_cleared.insert(canonical, ());
                }
                Some(raw_chord) => {
                    let Some(chord) = ShortcutChord::parse(raw_chord) else {
                        continue;
                    };
                    if !chord.is_valid_palette_chord() || reserved_system_action(chord).is_some() {
                        continue;
                    }
                    if by_chord.contains_key(&chord) || by_source.contains_key(canonical) {
                        continue;
                    }
                    by_source.insert(canonical, chord);
                    by_chord.insert(chord, canonical);
                }
            }
        }

        for &(raw_source, default_chord) in DEFAULT_PALETTE_SHORTCUTS {
            let Some(canonical) = palette::canonical_source(raw_source) else {
                continue;
            };
            if explicitly_cleared.contains_key(canonical)
                || by_source.contains_key(canonical)
                || by_chord.contains_key(&default_chord)
            {
                continue;
            }
            by_source.insert(canonical, default_chord);
            by_chord.insert(default_chord, canonical);
        }

        Self {
            by_source,
            by_chord,
        }
    }

    /// Returns `true` when every binding matches [`DEFAULT_PALETTE_SHORTCUTS`] exactly.
    pub(crate) fn is_default(&self) -> bool {
        self.to_overrides().is_empty()
    }

    /// Returns the number of currently assigned palette shortcuts.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.by_source.len()
    }

    /// Returns the assigned shortcut chord for `source`, if any.
    pub(crate) fn binding_for(&self, source: &str) -> Option<ShortcutChord> {
        self.by_source.get(source).copied()
    }

    /// Returns the canonical `PALETTE` `source` bound to `chord`, if any.
    pub(crate) fn source_for(&self, chord: ShortcutChord) -> Option<&'static str> {
        self.by_chord.get(&chord).copied()
    }

    /// Checks whether assigning `chord` to `target_source` collides with a system or palette entry.
    pub(crate) fn check_assignment(
        &self,
        target_source: &'static str,
        chord: ShortcutChord,
    ) -> Result<(), ShortcutConflict> {
        if let Some(system_action) = reserved_system_action(chord) {
            return Err(ShortcutConflict::System(system_action));
        }
        if let Some(existing_source) = self.source_for(chord) {
            if existing_source != target_source {
                return Err(ShortcutConflict::Palette(existing_source));
            }
        }
        Ok(())
    }

    /// Assigns `chord` to `target_source`, clearing any previous binding held by `target_source`
    /// or currently mapped to `chord` on another palette entry.
    pub(crate) fn replace_binding(&mut self, target_source: &'static str, chord: ShortcutChord) {
        if let Some(old_chord) = self.by_source.remove(target_source) {
            self.by_chord.remove(&old_chord);
        }
        if let Some(previous_owner) = self.by_chord.remove(&chord) {
            self.by_source.remove(previous_owner);
        }
        self.by_source.insert(target_source, chord);
        self.by_chord.insert(chord, target_source);
    }

    /// Swaps shortcuts between `target_source` (which must currently have a binding) and
    /// `other_source` (currently holding `other_chord`), or replaces when `target_source` has none.
    pub(crate) fn swap_bindings(
        &mut self,
        target_source: &'static str,
        other_source: &'static str,
    ) {
        if target_source == other_source {
            return;
        }
        let target_old = self.by_source.remove(target_source);
        let other_old = self.by_source.remove(other_source);
        if let Some(chord) = target_old {
            self.by_chord.remove(&chord);
        }
        if let Some(chord) = other_old {
            self.by_chord.remove(&chord);
        }
        if let Some(new_for_target) = other_old {
            self.by_source.insert(target_source, new_for_target);
            self.by_chord.insert(new_for_target, target_source);
        }
        if let Some(new_for_other) = target_old {
            self.by_source.insert(other_source, new_for_other);
            self.by_chord.insert(new_for_other, other_source);
        }
    }

    /// Removes any shortcut bound to `source`.
    pub(crate) fn clear_binding(&mut self, source: &'static str) {
        if let Some(old_chord) = self.by_source.remove(source) {
            self.by_chord.remove(&old_chord);
        }
    }

    /// Exports minimal persisted overrides relative to [`DEFAULT_PALETTE_SHORTCUTS`].
    pub(crate) fn to_overrides(&self) -> BTreeMap<String, Option<String>> {
        let mut overrides = BTreeMap::new();
        let mut defaults_by_source = HashMap::with_capacity(DEFAULT_PALETTE_SHORTCUTS.len());
        for &(raw_source, default_chord) in DEFAULT_PALETTE_SHORTCUTS {
            if let Some(canonical) = palette::canonical_source(raw_source) {
                defaults_by_source.insert(canonical, default_chord);
            }
        }

        for (&canonical, &default_chord) in &defaults_by_source {
            match self.by_source.get(canonical).copied() {
                Some(active_chord) if active_chord == default_chord => {}
                Some(active_chord) => {
                    overrides.insert(
                        canonical.to_owned(),
                        Some(active_chord.to_canonical_string()),
                    );
                }
                None => {
                    overrides.insert(canonical.to_owned(), None);
                }
            }
        }

        for (&canonical, &active_chord) in &self.by_source {
            if !defaults_by_source.contains_key(canonical) {
                overrides.insert(
                    canonical.to_owned(),
                    Some(active_chord.to_canonical_string()),
                );
            }
        }

        overrides
    }
}

/// Returns the fixed system command name if `chord` is reserved, or `None` otherwise.
pub(crate) fn reserved_system_action(chord: ShortcutChord) -> Option<&'static str> {
    RESERVED_SYSTEM_CHORDS
        .iter()
        .find(|&&(_, reserved)| reserved == chord)
        .map(|&(label, _)| label)
}

/// Normalizes persisted shortcut overrides by resolving and re-exporting minimal clean overrides.
pub(crate) fn sanitize_overrides(
    raw: &BTreeMap<String, Option<String>>,
) -> BTreeMap<String, Option<String>> {
    ShortcutMap::from_overrides(raw).to_overrides()
}

/// Looks up the human-readable palette label for a canonical `PALETTE` `source`.
pub(crate) fn palette_label(source: &str) -> &'static str {
    palette::PALETTE
        .iter()
        .find(|(_, _, item_source)| *item_source == source)
        .map_or("Palette template", |(_, label, _)| *label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn default_palette_shortcuts_match_valid_disjoint_canonical_entries() {
        let mut seen_sources = HashSet::new();
        let mut seen_chords = HashSet::new();
        for &(raw_source, chord) in DEFAULT_PALETTE_SHORTCUTS {
            let canonical = palette::canonical_source(raw_source)
                .unwrap_or_else(|| panic!("missing canonical palette entry for {raw_source}"));
            assert!(
                seen_sources.insert(canonical),
                "duplicate default source {canonical}"
            );
            assert!(
                chord.is_valid_palette_chord(),
                "invalid default chord {chord} for {canonical}"
            );
            assert_eq!(
                reserved_system_action(chord),
                None,
                "default chord {chord} collides with system command"
            );
            assert!(seen_chords.insert(chord), "duplicate default chord {chord}");
        }

        let map = ShortcutMap::default_map();
        assert_eq!(map.len(), DEFAULT_PALETTE_SHORTCUTS.len());
        assert!(map.is_default());
    }

    #[test]
    fn chord_round_trip_and_palette_validation() {
        for chord in [
            ShortcutChord::cmd(ShortcutKey::F),
            ShortcutChord::cmd_shift(ShortcutKey::R),
            ShortcutChord::new(true, true, false, ShortcutKey::V),
            ShortcutChord::new(false, true, true, ShortcutKey::OpenBracket),
            ShortcutChord::new(false, false, false, ShortcutKey::F4),
            ShortcutChord::cmd(ShortcutKey::Plus),
        ] {
            let text = chord.to_canonical_string();
            let parsed =
                ShortcutChord::parse(&text).unwrap_or_else(|| panic!("failed to re-parse {text}"));
            assert_eq!(parsed, chord);
            assert!(parsed.is_valid_palette_chord());
        }

        // Bare and shift-only printable chords are rejected for palette bindings.
        for bare in ["F", "Shift+F", "/", "9", "Shift+["] {
            let parsed = ShortcutChord::parse(bare).expect("parseable bare chord");
            assert!(!parsed.is_valid_palette_chord(), "{bare} must be rejected");
        }

        // Unsupported, malformed, or preserved editing/navigation strings fail to parse.
        for malformed in [
            "",
            "Ctrl+",
            "Ctrl+Ctrl+F",
            "Ctrl+UnknownKey",
            "Ctrl+Tab",
            "Tab",
            "Shift+Tab",
            "Enter",
            "Space",
            "Esc",
            "Ctrl+ArrowLeft",
            "Ctrl+ArrowRight",
            "Ctrl+Backspace",
            "Ctrl+Delete",
            "Ctrl+Home",
            "Ctrl+End",
            "Ctrl+Insert",
            "Shift+Insert",
            "Shift+Delete",
        ] {
            assert_eq!(ShortcutChord::parse(malformed), None, "{malformed}");
        }

        let mut raw_overrides = BTreeMap::new();
        let binom = palette::canonical_source(r"\binom{#?}{#?}").unwrap();
        for reserved_raw in [
            "Ctrl+ArrowLeft",
            "Ctrl+Backspace",
            "Ctrl+Home",
            "Ctrl+Insert",
            "Shift+Insert",
            "Shift+Delete",
            "Tab",
        ] {
            raw_overrides.insert(binom.to_owned(), Some(reserved_raw.to_owned()));
            let loaded = ShortcutMap::from_overrides(&raw_overrides);
            assert_eq!(loaded.binding_for(binom), None, "{reserved_raw}");
        }
    }

    #[test]
    fn conflict_detection_distinguishes_system_and_palette_collisions() {
        let mut map = ShortcutMap::default_map();
        let frac = palette::canonical_source(r"\frac{#?}{#?}").unwrap();
        let sqrt = palette::canonical_source(r"\sqrt{#?}").unwrap();
        let overarrow_src = palette::canonical_source(r"\overrightarrow{#?}").unwrap();

        assert_eq!(
            map.check_assignment(overarrow_src, ShortcutChord::cmd(ShortcutKey::S)),
            Err(ShortcutConflict::System("Save equation"))
        );
        assert_eq!(
            map.check_assignment(overarrow_src, ShortcutChord::cmd_shift(ShortcutKey::Z)),
            Err(ShortcutConflict::System("Redo"))
        );
        assert_eq!(
            map.check_assignment(overarrow_src, ShortcutChord::cmd(ShortcutKey::F)),
            Err(ShortcutConflict::Palette(frac))
        );
        // Re-assigning the same chord on its current owner is conflict-free.
        assert_eq!(
            map.check_assignment(frac, ShortcutChord::cmd(ShortcutKey::F)),
            Ok(())
        );

        // Swap exchanges bindings between two entries cleanly.
        map.swap_bindings(frac, sqrt);
        assert_eq!(
            map.binding_for(frac),
            Some(ShortcutChord::cmd(ShortcutKey::R))
        );
        assert_eq!(
            map.binding_for(sqrt),
            Some(ShortcutChord::cmd(ShortcutKey::F))
        );
        assert!(!map.is_default());

        // Replace moves chord to target and clears previous owner.
        map.replace_binding(overarrow_src, ShortcutChord::cmd(ShortcutKey::F));
        assert_eq!(
            map.binding_for(overarrow_src),
            Some(ShortcutChord::cmd(ShortcutKey::F))
        );
        assert_eq!(map.binding_for(sqrt), None);
    }
}
