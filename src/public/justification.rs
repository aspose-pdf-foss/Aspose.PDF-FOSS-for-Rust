//! `Aspose.Pdf.Annotations.Justification` — annotation enum, per `capabilities/justification.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `Justification` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum Justification {
    /// `Left`.
    #[default]
    Left = 0,
    /// `Center`.
    Center = 1,
    /// `Right`.
    Right = 2,
}
