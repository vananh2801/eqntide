//! Public settings and result types of a conversion run.

/// How each converted equation is written into `document.xml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmbedMode {
    /// A bare MathType OLE object with a rendered preview picture — exactly
    /// what Word writes for a MathType equation. Word, WPS and LibreOffice all
    /// *show* it (as the picture); MathType can edit it. The original OMML is
    /// replaced.
    #[default]
    Preview,
    /// Keep the original OMML visible: wrap the OLE object and the OMML in an
    /// `mc:AlternateContent` whose `Requires` makes Word choose the OMML. The
    /// OLE object is stored but never displayed.
    OmmlFallback,
}

/// Picture format of the preview drawn for each MathType object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewFormat {
    /// Vector EMF: sharp at any zoom. What Word and WPS prefer for OLE previews.
    #[default]
    Emf,
    /// Raster PNG (288 dpi): works everywhere but gets blurry when enlarged.
    Png,
}

/// Conversion settings for OMML → MathType (`convert*`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// How equations are embedded.
    pub mode: EmbedMode,
    /// Font size (points) the preview pictures are drawn at.
    pub font_pt: f32,
    /// Picture format of the previews (ignored in [`EmbedMode::OmmlFallback`]).
    pub preview: PreviewFormat,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: EmbedMode::Preview,
            font_pt: 12.0,
            preview: PreviewFormat::Emf,
        }
    }
}

/// Conversion settings for MathType → OMML (`convert_to_omml*`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToOmmlOptions {
    /// Optional math font name (e.g. `"Cambria Math"`) to apply to `<m:r>` runs.
    pub math_font: Option<String>,
}

/// Outcome of a conversion run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConversionSummary {
    /// Equations found in the document.
    pub found: usize,
    /// Equations successfully converted and replaced.
    pub converted: usize,
    /// Equations left untouched because there was nothing to convert
    /// (empty content, or an unsupported layout — see the log).
    pub skipped: usize,
    /// Equations that failed to convert (left in their original form).
    pub errors: usize,
}
