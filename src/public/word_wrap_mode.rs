//! `Aspose.Pdf.Facades.WordWrapMode` — text word-wrap mode, per
//! `capabilities/word_wrap_mode.yaml`. Ordinals match canonical 26.4.0.

/// A text word-wrap mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum WordWrapMode {
    /// The default wrap mode.
    #[default]
    Default = 0,
    /// Wrap by whole words.
    ByWords = 1,
}
