//! `Aspose.Pdf.Text.TextEditOptions.LanguageTransformation` — how
//! language-specific glyph forms are transformed while editing text,
//! per `capabilities/language_transformation.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// How language-specific glyph forms are transformed while editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum LanguageTransformation {
    /// Default language transformation.
    #[default]
    Default = 0,
    /// Keep glyphs exactly as displayed in the source.
    ExactlyAsISee = 1,
    /// Apply no language transformation.
    None = 2,
}
