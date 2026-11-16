//! `Aspose.Pdf.Annotations.LineIntent` — annotation enum, per `capabilities/line_intent.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// What a line annotation is being used as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum LineIntent {
    /// No intent; nothing is written.
    #[default]
    Undefined = 0,
    /// An arrow pointing at something.
    LineArrow = 1,
    /// A dimension line measuring a distance.
    LineDimension = 2,
}
