//! `Aspose.Pdf.Annotations.FreeTextIntent` — annotation enum, per `capabilities/free_text_intent.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `FreeTextIntent` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FreeTextIntent {
    /// `Undefined`.
    #[default]
    Undefined = 0,
    /// `FreeTextCallout`.
    FreeTextCallout = 1,
    /// `FreeTextTypeWriter`.
    FreeTextTypeWriter = 2,
}
