//! `Aspose.Pdf.Annotations.CapStyle` — annotation enum, per `capabilities/cap_style.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `CapStyle` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum CapStyle {
    /// `Rectangular`.
    #[default]
    Rectangular = 1,
    /// `Rounded`.
    Rounded = 2,
}
