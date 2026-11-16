//! `Aspose.Pdf.Annotations.TextAlignment` — annotation enum, per `capabilities/text_alignment.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Horizontal alignment carried by an annotation or its text style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum TextAlignment {
    /// Flush with the left edge.
    #[default]
    Left = 0,
    /// Centred.
    Center = 1,
    /// Flush with the right edge.
    Right = 2,
}
