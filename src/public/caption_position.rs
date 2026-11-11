//! `Aspose.Pdf.Annotations.CaptionPosition` — annotation enum, per `capabilities/caption_position.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `CaptionPosition` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum CaptionPosition {
    /// `Inline`.
    #[default]
    Inline = 0,
    /// `Top`.
    Top = 1,
}
