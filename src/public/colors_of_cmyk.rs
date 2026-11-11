//! `Aspose.Pdf.Annotations.ColorsOfCMYK` — annotation enum, per `capabilities/colors_of_cmyk.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `ColorsOfCMYK` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
#[allow(clippy::upper_case_acronyms)]
pub enum ColorsOfCMYK {
    /// `Cyan`.
    #[default]
    Cyan = 0,
    /// `Magenta`.
    Magenta = 1,
    /// `Yellow`.
    Yellow = 2,
    /// `Black`.
    Black = 3,
}
