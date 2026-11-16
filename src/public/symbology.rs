//! `Aspose.Pdf.Forms.Symbology` — forms enum, per `capabilities/symbology.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The barcode symbology a barcode field encodes its value in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
#[allow(clippy::upper_case_acronyms)]
pub enum Symbology {
    /// The PDF417 stacked linear barcode.
    #[default]
    PDF417 = 0,
    /// The QR Code matrix barcode.
    QRCode = 1,
    /// The Data Matrix matrix barcode.
    DataMatrix = 2,
}
