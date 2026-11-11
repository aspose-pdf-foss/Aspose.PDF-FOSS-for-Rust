//! `Aspose.Pdf.Forms.Symbology` — forms enum, per `capabilities/symbology.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `Symbology` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
#[allow(clippy::upper_case_acronyms)]
pub enum Symbology {
    /// `PDF417`.
    #[default]
    PDF417 = 0,
    /// `QRCode`.
    QRCode = 1,
    /// `DataMatrix`.
    DataMatrix = 2,
}
