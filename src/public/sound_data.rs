//! `Aspose.Pdf.Annotations.SoundData` — the audio parameters of a
//! [`SoundAnnotation`](crate::SoundAnnotation), per
//! `capabilities/sound_data.yaml`.
//!
//! Carries the sample-rate / channel / bit-depth / encoding metadata of the
//! annotation's sound. The canonical `Contents` stream getter (the raw sample
//! bytes) is dropped: the in-memory model holds only this metadata — a sound
//! built from [`SoundSampleData`] supplies no byte
//! samples — so there is no backing byte source to expose.
//!
//! **Carried, not written.** These parameters belong on the `/Sound`
//! stream's own dictionary (§12.5.6.16), and a stream is a second
//! object — the annotation writer emits one dictionary per annotation,
//! so there is nothing for a sample rate or a channel count to be
//! written onto. The annotation's `/Name` icon IS written; see
//! [`SoundAnnotation`](crate::SoundAnnotation) for that split.

use crate::public::sound_encoding::SoundEncoding;
use crate::public::sound_sample_data::SoundSampleData;
use crate::public::sound_sample_data_encoding_format::SoundSampleDataEncodingFormat;

/// The audio parameters of a sound annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SoundData {
    rate: i32,
    channels: i32,
    bits: i32,
    encoding: SoundEncoding,
}

impl SoundData {
    /// Derive sound data from the sample parameters supplied when creating the
    /// annotation.
    pub(crate) fn from_sample_data(sample: &SoundSampleData) -> Self {
        Self {
            rate: sample.sampling_rate() as i32,
            channels: sample.number_of_sound_channels(),
            bits: sample.bits_per_channel(),
            encoding: match sample.encoding_format() {
                SoundSampleDataEncodingFormat::Raw => SoundEncoding::Raw,
                SoundSampleDataEncodingFormat::Signed => SoundEncoding::Signed,
                SoundSampleDataEncodingFormat::MuLaw => SoundEncoding::MuLaw,
                SoundSampleDataEncodingFormat::ALaw => SoundEncoding::ALaw,
            },
        }
    }

    /// The sampling rate, in samples per second.
    pub fn rate(&self) -> i32 {
        self.rate
    }
    /// Set the sampling rate.
    pub fn set_rate(&mut self, value: i32) {
        self.rate = value;
    }

    /// The number of sound channels.
    pub fn channels(&self) -> i32 {
        self.channels
    }
    /// Set the number of sound channels.
    pub fn set_channels(&mut self, value: i32) {
        self.channels = value;
    }

    /// The number of bits per sample.
    pub fn bits(&self) -> i32 {
        self.bits
    }
    /// Set the number of bits per sample.
    pub fn set_bits(&mut self, value: i32) {
        self.bits = value;
    }

    /// The sample encoding.
    pub fn encoding(&self) -> SoundEncoding {
        self.encoding
    }
    /// Set the sample encoding.
    pub fn set_encoding(&mut self, value: SoundEncoding) {
        self.encoding = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_setters() {
        let mut d = SoundData::default();
        assert_eq!(d.rate(), 0);
        assert_eq!(d.channels(), 0);
        assert_eq!(d.bits(), 0);
        assert_eq!(d.encoding(), SoundEncoding::Raw);
        d.set_rate(44_100);
        d.set_channels(2);
        d.set_bits(16);
        d.set_encoding(SoundEncoding::Signed);
        assert_eq!(d.rate(), 44_100);
        assert_eq!(d.channels(), 2);
        assert_eq!(d.bits(), 16);
        assert_eq!(d.encoding(), SoundEncoding::Signed);
    }

    #[test]
    fn from_sample_data_maps_fields() {
        let sample =
            SoundSampleData::with_encoding(8_000, 1, 8, SoundSampleDataEncodingFormat::ALaw);
        let d = SoundData::from_sample_data(&sample);
        assert_eq!(d.rate(), 8_000);
        assert_eq!(d.channels(), 1);
        assert_eq!(d.bits(), 8);
        assert_eq!(d.encoding(), SoundEncoding::ALaw);
    }
}
