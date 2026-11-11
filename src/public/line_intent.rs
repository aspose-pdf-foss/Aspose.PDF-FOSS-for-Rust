//! `Aspose.Pdf.Annotations.LineIntent` — annotation enum, per `capabilities/line_intent.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `LineIntent` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum LineIntent {
    /// `Undefined`.
    #[default]
    Undefined = 0,
    /// `LineArrow`.
    LineArrow = 1,
    /// `LineDimension`.
    LineDimension = 2,
}
