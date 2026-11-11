//! `Aspose.Pdf.Annotations.RichTextFontStyles` — annotation enum, per `capabilities/rich_text_font_styles.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `RichTextFontStyles` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum RichTextFontStyles {
    /// `ClearExisting`.
    #[default]
    ClearExisting = 1,
    /// `Bold`.
    Bold = 2,
    /// `Italic`.
    Italic = 4,
    /// `Underline`.
    Underline = 8,
}
