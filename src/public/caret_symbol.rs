//! `Aspose.Pdf.Annotations.CaretSymbol` — annotation enum, per `capabilities/caret_symbol.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `CaretSymbol` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum CaretSymbol {
    /// `None`.
    #[default]
    None = 0,
    /// `Paragraph`.
    Paragraph = 1,
}
