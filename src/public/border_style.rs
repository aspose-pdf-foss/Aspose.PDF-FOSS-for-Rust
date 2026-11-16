//! `Aspose.Pdf.Annotations.BorderStyle` — annotation enum, per `capabilities/border_style.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// How an annotation's or form field's border line is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum BorderStyle {
    /// A solid rectangle around the whole area. Written as `S`.
    #[default]
    Solid = 0,
    /// A dashed rectangle, the pattern taken from the dash array. Written as `D`.
    Dashed = 1,
    /// A solid border given an embossed, raised look. Written as `B`.
    Beveled = 2,
    /// A solid border given an engraved, recessed look. Written as `I`.
    Inset = 3,
    /// A single line along the bottom edge only. Written as `U`.
    Underline = 4,
}
