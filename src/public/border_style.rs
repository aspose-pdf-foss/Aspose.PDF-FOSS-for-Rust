//! `Aspose.Pdf.Annotations.BorderStyle` — annotation enum, per `capabilities/border_style.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `BorderStyle` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum BorderStyle {
    /// `Solid`.
    #[default]
    Solid = 0,
    /// `Dashed`.
    Dashed = 1,
    /// `Beveled`.
    Beveled = 2,
    /// `Inset`.
    Inset = 3,
    /// `Underline`.
    Underline = 4,
}
