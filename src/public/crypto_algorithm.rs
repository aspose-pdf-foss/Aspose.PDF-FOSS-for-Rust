//! `Aspose.Pdf.CryptoAlgorithm` — document-encryption cipher
//! selector, per `capabilities/crypto_algorithm.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.CryptoAlgorithm` enum. The reference ships the four
//! built-in ciphers; the canonical `Custom` value (a pluggable
//! external security handler) is dropped to match the C++
//! reference selection.

/// Encryption cipher applied by the standard security handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum CryptoAlgorithm {
    /// RC4 with a 40-bit key (PDF security handler revision 2).
    #[default]
    Rc4x40 = 0,
    /// RC4 with a 128-bit key (revision 3).
    Rc4x128 = 1,
    /// AES with a 128-bit key (revision 4).
    Aesx128 = 2,
    /// AES with a 256-bit key (revision 6).
    Aesx256 = 3,
}
