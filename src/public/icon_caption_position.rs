//! `Aspose.Pdf.Forms.IconCaptionPosition` — forms enum, per `capabilities/icon_caption_position.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `IconCaptionPosition` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum IconCaptionPosition {
    /// `NoIcon`.
    #[default]
    NoIcon = 0,
    /// `NoCaption`.
    NoCaption = 1,
    /// `CaptionBelowIcon`.
    CaptionBelowIcon = 2,
    /// `CaptionAboveIcon`.
    CaptionAboveIcon = 3,
    /// `CaptionToTheRight`.
    CaptionToTheRight = 4,
    /// `CaptionToTheLeft`.
    CaptionToTheLeft = 5,
    /// `CaptionOverlaid`.
    CaptionOverlaid = 6,
}
