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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToOmmlOptions {
    /// Optional math font name (defaults to `"Cambria Math"`) to apply to `<m:r>` runs and `<m:ctrlPr>`.
    pub math_font: Option<String>,
}

impl Default for ToOmmlOptions {
    fn default() -> Self {
        ToOmmlOptions {
            math_font: Some("Cambria Math".into()),
        }
    }
}

/// Target format for upgrading legacy MathType equations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpgradeTarget {
    /// Modern MathType 7 format (default): upgrade MTEF 3.0/WMF/Equation.3 to genuine MathType 5.0 OLE (`Equation.DSMT4`)
    /// with sharp vector EMF previews embedding MTEF in `EMR_COMMENT`.
    #[default]
    ModernMathType,
    /// Modern Add-in format: convert legacy MathType/Equation 3.0 to native Word OMML (<m:oMath>).
    ModernOmml,
}

/// Settings for upgrading legacy MathType equations in a `.docx`.
#[derive(Debug, Clone, PartialEq)]
pub struct UpgradeOptions {
    /// Target format for the upgraded equations.
    pub target: UpgradeTarget,
    /// Font size (points) for re-rendered preview pictures (used when `target == ModernMathType`).
    pub font_pt: f32,
    /// Optional math font name (e.g. `"Cambria Math"`) applied when `target == ModernOmml`.
    pub math_font: Option<String>,
}

impl Default for UpgradeOptions {
    fn default() -> Self {
        UpgradeOptions {
            target: UpgradeTarget::ModernMathType,
            font_pt: 12.0,
            math_font: Some("Cambria Math".into()),
        }
    }
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
