//! `Aspose.Pdf.Annotations.SoundIcon` — annotation enum, per `capabilities/sound_icon.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The icon a sound annotation is shown with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SoundIcon {
    /// A loudspeaker icon.
    #[default]
    Speaker = 0,
    /// A microphone icon.
    Mic = 1,
}
