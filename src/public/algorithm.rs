//! `Aspose.Pdf.Facades.Algorithm` — the cipher family used by
//! `PdfFileSecurity`, per `capabilities/algorithm.yaml`. Ordinals match
//! canonical 26.4.0.

/// A document-encryption cipher family.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum Algorithm {
    /// RC4 stream cipher.
    #[default]
    RC4 = 0,
    /// AES block cipher.
    AES = 1,
}
