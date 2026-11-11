//! `Aspose.Pdf.Text.TextReplaceOptions.ReplaceAdjustment` — how the
//! surrounding layout is adjusted after a text replacement, per
//! `capabilities/replace_adjustment.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 flags enum.

/// How the surrounding layout is adjusted after a text replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ReplaceAdjustment {
    /// No adjustment.
    #[default]
    None = 0,
    /// Adjust the width of spaces to fit.
    AdjustSpaceWidth = 1,
    /// Hyphenate whole words to fit.
    WholeWordsHyphenation = 2,
    /// Form-filling adjustment mode.
    IsFormFillingMode = 4,
    /// Shift the rest of the line.
    ShiftRestOfLine = 8,
}
