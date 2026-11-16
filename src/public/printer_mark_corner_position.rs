//! `Aspose.Pdf.Annotations.PrinterMarkCornerPosition` — annotation enum, per `capabilities/printer_mark_corner_position.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Which corner of the page a corner printer mark is placed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PrinterMarkCornerPosition {
    /// The top-left corner of the page.
    #[default]
    TopLeft = 0,
    /// The top-right corner of the page.
    TopRight = 1,
    /// The bottom-left corner of the page.
    BottomLeft = 2,
    /// The bottom-right corner of the page.
    BottomRight = 3,
}
