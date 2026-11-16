//! `Aspose.Pdf.Annotations.ColorsOfCMYK` — annotation enum, per `capabilities/colors_of_cmyk.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Which process colour a colour-bar printer mark stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
#[allow(clippy::upper_case_acronyms)]
pub enum ColorsOfCMYK {
    /// The cyan process plate.
    #[default]
    Cyan = 0,
    /// The magenta process plate.
    Magenta = 1,
    /// The yellow process plate.
    Yellow = 2,
    /// The black process plate.
    Black = 3,
}
