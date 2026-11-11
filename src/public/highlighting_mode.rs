//! `Aspose.Pdf.Annotations.HighlightingMode` — annotation enum, per `capabilities/highlighting_mode.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `HighlightingMode` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum HighlightingMode {
    /// `None`.
    #[default]
    None = 0,
    /// `Invert`.
    Invert = 1,
    /// `Outline`.
    Outline = 2,
    /// `Push`.
    Push = 3,
    /// `Toggle`.
    Toggle = 4,
}
