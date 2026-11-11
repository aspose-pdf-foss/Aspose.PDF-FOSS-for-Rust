//! `Aspose.Pdf.Annotations.PrinterMarksKind` — annotation enum, per `capabilities/printer_marks_kind.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `PrinterMarksKind` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PrinterMarksKind {
    /// `None`.
    #[default]
    None = 0,
    /// `TrimMarks`.
    TrimMarks = 1,
    /// `BleedMarks`.
    BleedMarks = 2,
    /// `RegistrationMarks`.
    RegistrationMarks = 4,
    /// `ColorBars`.
    ColorBars = 8,
    /// `PageInformation`.
    PageInformation = 16,
    /// `All`.
    All = 31,
}
