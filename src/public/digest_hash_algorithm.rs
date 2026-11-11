//! `Aspose.Pdf.DigestHashAlgorithm` — the message-digest hash used when
//! signing a document, per `capabilities/digest_hash_algorithm.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The message-digest hash algorithm used to sign a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum DigestHashAlgorithm {
    /// Select the hash automatically.
    #[default]
    Auto = 0,
    /// SHA-1.
    Sha1 = 1,
    /// SHA-256.
    Sha256 = 2,
    /// SHA-384.
    Sha384 = 3,
    /// SHA-512.
    Sha512 = 4,
    /// SHA3-256.
    Sha3_256 = 5,
    /// SHA3-384.
    Sha3_384 = 6,
    /// SHA3-512.
    Sha3_512 = 7,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(DigestHashAlgorithm::Auto as i32, 0);
        assert_eq!(DigestHashAlgorithm::Sha1 as i32, 1);
        assert_eq!(DigestHashAlgorithm::Sha256 as i32, 2);
        assert_eq!(DigestHashAlgorithm::Sha384 as i32, 3);
        assert_eq!(DigestHashAlgorithm::Sha512 as i32, 4);
        assert_eq!(DigestHashAlgorithm::Sha3_256 as i32, 5);
        assert_eq!(DigestHashAlgorithm::Sha3_384 as i32, 6);
        assert_eq!(DigestHashAlgorithm::Sha3_512 as i32, 7);
    }

    #[test]
    fn default_is_auto() {
        assert_eq!(DigestHashAlgorithm::default(), DigestHashAlgorithm::Auto);
    }
}
