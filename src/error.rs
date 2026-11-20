//! Error type shared by every stage of the conversion pipeline.

use std::fmt;

/// Everything that can go wrong while converting a document.
#[derive(Debug)]
pub enum Error {
    /// Filesystem / stream I/O failure.
    Io(std::io::Error),
    /// The input is not a readable ZIP container (or the output could not be written).
    Zip(zip::result::ZipError),
    /// An XML part is malformed.
    Xml(String),
    /// A mandatory package part (e.g. `word/document.xml`) is missing.
    MissingPart(String),
    /// An OLE2 compound file or relationship is malformed or missing its equation stream.
    InvalidOle(String),
    /// An MTEF binary stream could not be decoded.
    InvalidMtef(String),
    /// The equation does not fit in the single-FAT-sector OLE container we write.
    EquationTooLarge {
        /// Size of the `Equation Native` stream in bytes.
        stream_len: usize,
    },
    /// Nesting is deeper than the converter is willing to recurse.
    TooDeep,
    /// The equation would need an absurdly large preview image.
    PreviewTooLarge {
        /// Image width in pixels.
        width: usize,
        /// Image height in pixels.
        height: usize,
    },
    /// The conversion was cancelled via cooperative cancellation signal.
    Cancelled,
}

/// Convenience alias used across the crate.
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::Zip(e) => write!(f, "{e}"),
            Error::Xml(msg) => write!(f, "malformed XML: {msg}"),
            Error::MissingPart(name) => write!(f, "missing package part: {name}"),
            Error::InvalidOle(msg) => write!(f, "invalid OLE object: {msg}"),
            Error::InvalidMtef(msg) => write!(f, "invalid MTEF stream: {msg}"),
            Error::EquationTooLarge { stream_len } => write!(
                f,
                "equation is too large for a single-FAT-sector OLE file ({stream_len} bytes)"
            ),
            Error::TooDeep => write!(f, "equation is nested too deeply"),
            Error::PreviewTooLarge { width, height } => write!(
                f,
                "equation preview would be too large ({width}x{height} px)"
            ),
            Error::Cancelled => write!(f, "conversion cancelled"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Zip(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error::Zip(e)
    }
}
