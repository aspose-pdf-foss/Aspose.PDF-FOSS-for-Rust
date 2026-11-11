//! `Aspose.Pdf.HorizontalAlignment` — horizontal placement
//! selector, per `capabilities/horizontal_alignment.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.HorizontalAlignment` enum.

/// Horizontal alignment of content within its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum HorizontalAlignment {
    /// No explicit alignment — inherit / leave as positioned.
    #[default]
    None = 0,
    /// Align to the left edge.
    Left = 1,
    /// Centre horizontally.
    Center = 2,
    /// Align to the right edge.
    Right = 3,
    /// Justify, leaving the last line ragged.
    Justify = 4,
    /// Justify including the last line.
    FullJustify = 5,
}
