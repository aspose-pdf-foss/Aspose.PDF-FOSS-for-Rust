//! `Aspose.Pdf.Text.TextEditOptions.FontReplace` — the font-replacement
//! behaviour applied while editing text, per
//! `capabilities/font_replace.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Font-replacement behaviour applied while editing text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FontReplace {
    /// Default font-replacement behaviour.
    #[default]
    Default = 0,
    /// Remove fonts left unused after replacement.
    RemoveUnusedFonts = 1,
}
