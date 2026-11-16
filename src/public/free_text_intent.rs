//! `Aspose.Pdf.Annotations.FreeTextIntent` — annotation enum, per `capabilities/free_text_intent.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// What a free text annotation is being used as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FreeTextIntent {
    /// No intent; nothing is written.
    #[default]
    Undefined = 0,
    /// A callout, drawn with a leader line to what it refers to.
    FreeTextCallout = 1,
    /// Typed text placed directly on the page, with no border.
    FreeTextTypeWriter = 2,
}
