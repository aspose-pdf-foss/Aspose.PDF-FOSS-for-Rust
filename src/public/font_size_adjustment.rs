//! `Aspose.Pdf.Text.TextReplaceOptions.FontSizeAdjustment` — how the
//! replacement font size is adjusted to fit, per
//! `capabilities/font_size_adjustment.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// How the replacement font size is adjusted to fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FontSizeAdjustment {
    /// No adjustment.
    #[default]
    None = 0,
    /// Shrink the font so the replacement text fits.
    ShrinkToFit = 1,
    /// Scale the font to fill the available space.
    ScaleToFill = 2,
}
