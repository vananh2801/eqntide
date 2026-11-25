//! Byte- and nibble-oriented cursor over an in-memory MTEF binary stream.

use super::consts::{REC_FULL, REC_LINE, REC_TMPL};

/// Stateful cursor supporting byte, little-endian 16-bit, variable-length unsigned,
/// and nibble reads over MTEF v3/v5 payloads.
#[derive(Clone)]
pub(super) struct ByteStream<'a> {
    data: &'a [u8],
    pos: usize,
    nibble: Option<u8>,
}

impl<'a> ByteStream<'a> {
    /// Construct a cursor positioned at the beginning of `data`.
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            nibble: None,
        }
    }

    /// Return the number of unread bytes remaining in the buffer.
    pub(super) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Return the current byte offset into the buffer.
    pub(super) fn position(&self) -> usize {
        self.pos
    }

    /// Reposition the cursor to `pos` and clear any pending half-byte nibble state.
    pub(super) fn set_position(&mut self, pos: usize) {
        self.pos = pos.min(self.data.len());
        self.nibble = None;
    }

    /// Read an unsigned 8-bit integer, combining with a pending nibble if mid-byte.
    pub(super) fn uint8(&mut self) -> Option<u8> {
        if let Some(low) = self.nibble.take() {
            let high = *self.data.get(self.pos)?;
            self.pos += 1;
            return Some((high << 4) | low);
        }
        let byte = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(byte)
    }

    /// Read a signed 8-bit integer.
    pub(super) fn int8(&mut self) -> Option<i8> {
        self.uint8().map(|byte| byte as i8)
    }

    /// Read a little-endian unsigned 16-bit integer.
    pub(super) fn mtef16(&mut self) -> Option<u16> {
        let lo = self.uint8()? as u16;
        let hi = self.uint8()? as u16;
        Some((hi << 8) | lo)
    }

    /// Read an MTEF variable-length unsigned integer (`< 0xFF` inline, else `u16`).
    pub(super) fn mt_uint(&mut self) -> Option<usize> {
        let first = self.uint8()?;
        if first < 0xFF {
            Some(first as usize)
        } else {
            self.mtef16().map(|value| value as usize)
        }
    }

    /// Read the next 4-bit nibble (high nibble first, then low nibble).
    pub(super) fn nibble(&mut self) -> Option<u8> {
        if let Some(low) = self.nibble.take() {
            return Some(low);
        }
        let byte = *self.data.get(self.pos)?;
        self.pos += 1;
        self.nibble = Some(byte & 0x0F);
        Some((byte >> 4) & 0x0F)
    }

    /// Discard any trailing low nibble so the next read starts on a byte boundary.
    pub(super) fn align_byte(&mut self) {
        self.nibble = None;
    }

    /// Step backward by one byte and reset nibble alignment.
    pub(super) fn unread_byte(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
            self.nibble = None;
        }
    }

    /// Advance the cursor by `count` bytes, clamped to the end of the buffer.
    pub(super) fn skip_bytes(&mut self, count: usize) {
        self.nibble = None;
        self.pos = self.pos.saturating_add(count).min(self.data.len());
    }

    /// Skip a NUL-terminated string in the stream.
    pub(super) fn skip_c_string(&mut self) {
        while let Some(byte) = self.uint8() {
            if byte == 0 {
                break;
            }
        }
    }

    /// Fallback scan ahead when malformed `EQN_PREFS` causes premature EOF, locating
    /// the next `FULL` marker followed by `LINE` or `TMPL`.
    pub(super) fn rescan_for_equation_body(&mut self) {
        let end = self.data.len();
        let pos = self.pos;
        for idx in pos..end.saturating_sub(3).min(pos + 150) {
            if (self.data[idx] & 0x1F) == REC_FULL {
                let next_record_type = self
                    .data
                    .get(idx + 1)
                    .map(|byte| byte & 0x1F)
                    .unwrap_or(0xFF);
                if next_record_type == REC_LINE || next_record_type == REC_TMPL {
                    self.pos = idx + 1;
                    self.nibble = None;
                    return;
                }
            }
        }
        self.pos = pos.max(end.saturating_sub(50));
        self.nibble = None;
    }
}
