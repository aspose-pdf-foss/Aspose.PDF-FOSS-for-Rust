//! `Aspose.Pdf.Annotations.SoundEncoding` — annotation enum, per
//! `capabilities/sound_encoding.yaml`.
//!
//! The sample-encoding scheme of a [`SoundData`](crate::SoundData) stream.
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `SoundEncoding` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SoundEncoding {
    /// `Raw` — unspecified or unsigned raw samples.
    #[default]
    Raw = 0,
    /// `Signed` — twos-complement signed samples.
    Signed = 1,
    /// `MuLaw` — mu-law companded samples.
    MuLaw = 2,
    /// `ALaw` — A-law companded samples.
    ALaw = 3,
}
