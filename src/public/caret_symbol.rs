//! `Aspose.Pdf.Annotations.CaretSymbol` — annotation enum, per `capabilities/caret_symbol.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The symbol a caret annotation draws, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum CaretSymbol {
    /// No symbol is drawn.
    #[default]
    None = 0,
    /// A pilcrow, marking a new paragraph.
    Paragraph = 1,
}
