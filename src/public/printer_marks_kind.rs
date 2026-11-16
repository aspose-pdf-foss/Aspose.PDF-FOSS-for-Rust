//! `Aspose.Pdf.Annotations.PrinterMarksKind` — annotation enum, per `capabilities/printer_marks_kind.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Which families of printer marks are meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PrinterMarksKind {
    /// No printer marks.
    #[default]
    None = 0,
    /// Marks showing where the page is trimmed.
    TrimMarks = 1,
    /// Marks showing the bleed boundary.
    BleedMarks = 2,
    /// Marks used to align the printing plates.
    RegistrationMarks = 4,
    /// Patches of the process colours, for press calibration.
    ColorBars = 8,
    /// Page identification text.
    PageInformation = 16,
    /// Every kind of printer mark.
    All = 31,
}
