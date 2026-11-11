//! `Aspose.Pdf.VerticalAlignment` — vertical placement selector,
//! per `capabilities/vertical_alignment.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.VerticalAlignment` enum.

/// Vertical alignment of content within its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum VerticalAlignment {
    /// No explicit alignment — inherit / leave as positioned.
    #[default]
    None = 0,
    /// Align to the top edge.
    Top = 1,
    /// Centre vertically.
    Center = 2,
    /// Align to the bottom edge.
    Bottom = 3,
}
