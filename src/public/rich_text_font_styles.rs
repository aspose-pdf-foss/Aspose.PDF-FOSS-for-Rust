//! `Aspose.Pdf.Annotations.RichTextFontStyles` — annotation enum, per `capabilities/rich_text_font_styles.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Font styles applied to a range of a free text annotation's rich text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum RichTextFontStyles {
    /// Drop the styling already on the range.
    #[default]
    ClearExisting = 1,
    /// Bold.
    Bold = 2,
    /// Italic.
    Italic = 4,
    /// Underlined.
    Underline = 8,
}
