//! `Aspose.Pdf.Annotations.CapStyle` — annotation enum, per `capabilities/cap_style.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The shape given to the ends of an ink annotation's strokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum CapStyle {
    /// Strokes end square.
    #[default]
    Rectangular = 1,
    /// Strokes end in a half-circle.
    Rounded = 2,
}
