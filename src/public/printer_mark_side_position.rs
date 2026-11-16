//! `Aspose.Pdf.Annotations.PrinterMarkSidePosition` — annotation enum, per `capabilities/printer_mark_side_position.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Which edge of the page a registration mark is placed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PrinterMarkSidePosition {
    /// Centred on the top edge of the page.
    #[default]
    Top = 0,
    /// Centred on the bottom edge of the page.
    Bottom = 1,
    /// Centred on the left edge of the page.
    Left = 2,
    /// Centred on the right edge of the page.
    Right = 3,
}
