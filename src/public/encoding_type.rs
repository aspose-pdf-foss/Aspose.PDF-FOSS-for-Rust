//! `Aspose.Pdf.Facades.EncodingType` — a form-field text encoding, per
//! `capabilities/encoding_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Facades.EncodingType` enum.

/// A form-field text encoding.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum EncodingType {
    /// Identity-H (horizontal CID).
    #[default]
    Identity_h = 0,
    /// Identity-V (vertical CID).
    Identity_v = 1,
    /// Windows code page 1250 (Central European).
    Cp1250 = 2,
    /// Windows code page 1252 (Western European).
    Cp1252 = 3,
    /// Windows code page 1257 (Baltic).
    Cp1257 = 4,
    /// WinAnsi.
    Winansi = 5,
    /// Mac Roman.
    Macroman = 6,
}
