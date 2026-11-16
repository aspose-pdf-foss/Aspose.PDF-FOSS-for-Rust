//! `Aspose.Pdf.Forms.IconCaptionPosition` — forms enum, per `capabilities/icon_caption_position.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// How a push button's icon and caption are arranged relative to each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum IconCaptionPosition {
    /// Caption only; no icon is drawn.
    #[default]
    NoIcon = 0,
    /// Icon only; no caption is drawn.
    NoCaption = 1,
    /// Caption below the icon.
    CaptionBelowIcon = 2,
    /// Caption above the icon.
    CaptionAboveIcon = 3,
    /// Caption to the right of the icon.
    CaptionToTheRight = 4,
    /// Caption to the left of the icon.
    CaptionToTheLeft = 5,
    /// Caption drawn over the icon.
    CaptionOverlaid = 6,
}
