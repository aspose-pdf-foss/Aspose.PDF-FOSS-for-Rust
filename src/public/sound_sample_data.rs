//! `Aspose.Pdf.Annotations.SoundSampleData` — audio-sample parameters
//! supplied when creating a [`SoundAnnotation`](crate::SoundAnnotation),
//! per `capabilities/sound_sample_data.yaml`.
//!
//! A plain parameter carrier: sampling rate, channel count, bit depth and
//! encoding format. The canonical shorter constructors leave the omitted
//! fields at their defaults — 1 channel, 8 bits per channel, `Raw` encoding.

use crate::public::sound_sample_data_encoding_format::SoundSampleDataEncodingFormat;

/// Audio-sample parameters for a sound annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundSampleData {
    sampling_rate: i64,
    number_of_sound_channels: i32,
    bits_per_channel: i32,
    encoding_format: SoundSampleDataEncodingFormat,
}

impl SoundSampleData {
    /// Construct from a sampling rate, defaulting to 1 channel, 8 bits per
    /// channel and `Raw` encoding.
    pub fn new(sampling_rate: i64) -> Self {
        Self {
            sampling_rate,
            number_of_sound_channels: 1,
            bits_per_channel: 8,
            encoding_format: SoundSampleDataEncodingFormat::Raw,
        }
    }

    /// Construct from a sampling rate and channel count, defaulting to 8 bits
    /// per channel and `Raw` encoding.
    pub fn with_channels(sampling_rate: i64, number_of_sound_channels: i32) -> Self {
        Self {
            number_of_sound_channels,
            ..Self::new(sampling_rate)
        }
    }

    /// Construct from a sampling rate, channel count and bit depth, defaulting
    /// to `Raw` encoding.
    pub fn with_bits(
        sampling_rate: i64,
        number_of_sound_channels: i32,
        bits_per_channel: i32,
    ) -> Self {
        Self {
            bits_per_channel,
            ..Self::with_channels(sampling_rate, number_of_sound_channels)
        }
    }

    /// Construct from a sampling rate, channel count, bit depth and encoding
    /// format.
    pub fn with_encoding(
        sampling_rate: i64,
        number_of_sound_channels: i32,
        bits_per_channel: i32,
        encoding_format: SoundSampleDataEncodingFormat,
    ) -> Self {
        Self {
            encoding_format,
            ..Self::with_bits(sampling_rate, number_of_sound_channels, bits_per_channel)
        }
    }

    /// The sampling rate, in samples per second.
    pub fn sampling_rate(&self) -> i64 {
        self.sampling_rate
    }
    /// Set the sampling rate.
    pub fn set_sampling_rate(&mut self, value: i64) {
        self.sampling_rate = value;
    }

    /// The number of sound channels.
    pub fn number_of_sound_channels(&self) -> i32 {
        self.number_of_sound_channels
    }
    /// Set the number of sound channels.
    pub fn set_number_of_sound_channels(&mut self, value: i32) {
        self.number_of_sound_channels = value;
    }

    /// The number of bits per channel.
    pub fn bits_per_channel(&self) -> i32 {
        self.bits_per_channel
    }
    /// Set the number of bits per channel.
    pub fn set_bits_per_channel(&mut self, value: i32) {
        self.bits_per_channel = value;
    }

    /// The encoding format.
    pub fn encoding_format(&self) -> SoundSampleDataEncodingFormat {
        self.encoding_format
    }
    /// Set the encoding format.
    pub fn set_encoding_format(&mut self, value: SoundSampleDataEncodingFormat) {
        self.encoding_format = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctor_defaults() {
        let s = SoundSampleData::new(44_100);
        assert_eq!(s.sampling_rate(), 44_100);
        assert_eq!(s.number_of_sound_channels(), 1);
        assert_eq!(s.bits_per_channel(), 8);
        assert_eq!(s.encoding_format(), SoundSampleDataEncodingFormat::Raw);
    }

    #[test]
    fn ctor_progressive_overrides() {
        let s = SoundSampleData::with_channels(22_050, 2);
        assert_eq!(s.number_of_sound_channels(), 2);
        assert_eq!(s.bits_per_channel(), 8);

        let s = SoundSampleData::with_bits(22_050, 2, 16);
        assert_eq!(s.bits_per_channel(), 16);
        assert_eq!(s.encoding_format(), SoundSampleDataEncodingFormat::Raw);

        let s = SoundSampleData::with_encoding(8_000, 1, 8, SoundSampleDataEncodingFormat::MuLaw);
        assert_eq!(s.encoding_format(), SoundSampleDataEncodingFormat::MuLaw);
    }
}
