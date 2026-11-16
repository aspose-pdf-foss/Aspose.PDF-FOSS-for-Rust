//! `Aspose.Pdf.Annotations.CaptionPosition` — annotation enum, per `capabilities/caption_position.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Where a line annotation's caption sits relative to the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum CaptionPosition {
    /// The caption sits in a gap in the line itself.
    #[default]
    Inline = 0,
    /// The caption sits above the line.
    Top = 1,
}
