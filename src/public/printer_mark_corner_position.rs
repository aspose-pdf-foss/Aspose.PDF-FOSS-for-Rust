//! `Aspose.Pdf.Annotations.PrinterMarkCornerPosition` — annotation enum, per `capabilities/printer_mark_corner_position.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `PrinterMarkCornerPosition` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PrinterMarkCornerPosition {
    /// `TopLeft`.
    #[default]
    TopLeft = 0,
    /// `TopRight`.
    TopRight = 1,
    /// `BottomLeft`.
    BottomLeft = 2,
    /// `BottomRight`.
    BottomRight = 3,
}
