//! `Aspose.Pdf.Annotations.PrinterMarkSidePosition` — annotation enum, per `capabilities/printer_mark_side_position.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `PrinterMarkSidePosition` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PrinterMarkSidePosition {
    /// `Top`.
    #[default]
    Top = 0,
    /// `Bottom`.
    Bottom = 1,
    /// `Left`.
    Left = 2,
    /// `Right`.
    Right = 3,
}
