//! `Aspose.Pdf.Annotations.BorderEffect` — annotation enum, per `capabilities/border_effect.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Whether an annotation's border is drawn plain or with a cloudy effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum BorderEffect {
    /// A plain border.
    #[default]
    None = 0,
    /// A border drawn as a series of arcs, like a cloud.
    Cloudy = 1,
}
