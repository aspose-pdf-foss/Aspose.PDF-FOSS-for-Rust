//! `Aspose.Pdf.Annotations.SoundSampleDataEncodingFormat` — annotation
//! enum, per `capabilities/sound_sample_data_encoding_format.yaml`.
//!
//! The encoding format supplied to [`SoundSampleData`](crate::SoundSampleData).
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `SoundSampleDataEncodingFormat` — see the canonical Aspose.PDF
/// documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SoundSampleDataEncodingFormat {
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
