//! `Aspose.Pdf.Annotations.Justification` — annotation enum, per `capabilities/justification.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Horizontal alignment of a free text annotation's or rich text field's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum Justification {
    /// Flush with the left edge.
    #[default]
    Left = 0,
    /// Centred.
    Center = 1,
    /// Flush with the right edge.
    Right = 2,
}
