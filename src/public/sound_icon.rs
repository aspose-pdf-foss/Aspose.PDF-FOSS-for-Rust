//! `Aspose.Pdf.Annotations.SoundIcon` — annotation enum, per `capabilities/sound_icon.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `SoundIcon` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SoundIcon {
    /// `Speaker`.
    #[default]
    Speaker = 0,
    /// `Mic`.
    Mic = 1,
}
