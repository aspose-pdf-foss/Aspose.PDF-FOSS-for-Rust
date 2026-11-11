//! `Aspose.Pdf.Annotations.TextAlignment` — annotation enum, per `capabilities/text_alignment.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `TextAlignment` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum TextAlignment {
    /// `Left`.
    #[default]
    Left = 0,
    /// `Center`.
    Center = 1,
    /// `Right`.
    Right = 2,
}
