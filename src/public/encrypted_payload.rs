//! `Aspose.Pdf.EncryptedPayload` — the encrypted-payload descriptor of a
//! file specification, per `capabilities/encrypted_payload.yaml`.
//!
//! Mirrors the PDF 2.0 `/EP` entry of a file-specification dictionary
//! (an "encrypted payload" — the wrapper that carries a ciphertext file
//! inside an otherwise-unencrypted document). Exposes the payload's
//! [`type_`](EncryptedPayload::type_) (`/Type`, always `EncryptedPayload`),
//! its crypto-filter [`subtype`](EncryptedPayload::subtype) (`/Subtype`),
//! and its [`version`](EncryptedPayload::version) (`/Version`). Reached via
//! [`FileSpecification::encrypted_payload`](crate::FileSpecification::encrypted_payload).

use crate::public::file_specification::FileSpecification;

/// The encrypted-payload descriptor of a file specification.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EncryptedPayload {
    type_: String,
    subtype: String,
    version: String,
}

impl EncryptedPayload {
    /// The encrypted payload described by `file_specification`'s `/EP`
    /// entry, or an empty descriptor when the specification carries none.
    pub fn new(file_specification: &FileSpecification) -> Self {
        file_specification
            .encrypted_payload()
            .cloned()
            .unwrap_or_default()
    }

    /// Build from the parsed `/EP` fields. Crate-internal — populated when
    /// a document's embedded files are loaded.
    pub(crate) fn from_parts(type_: String, subtype: String, version: String) -> Self {
        Self { type_, subtype, version }
    }

    /// The payload type (`/Type` — the PDF spec fixes this to
    /// `EncryptedPayload`).
    pub fn type_(&self) -> &str {
        &self.type_
    }

    /// The crypto-filter subtype (`/Subtype`) — the registered name of the
    /// scheme used to encrypt the payload.
    pub fn subtype(&self) -> &str {
        &self.subtype
    }

    /// The payload version (`/Version`).
    pub fn version(&self) -> &str {
        &self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_parts_exposes_fields() {
        let ep = EncryptedPayload::from_parts(
            "EncryptedPayload".into(),
            "AESV3".into(),
            "1".into(),
        );
        assert_eq!(ep.type_(), "EncryptedPayload");
        assert_eq!(ep.subtype(), "AESV3");
        assert_eq!(ep.version(), "1");
    }

    #[test]
    fn new_from_bare_spec_is_empty() {
        let fs = FileSpecification::new();
        let ep = EncryptedPayload::new(&fs);
        assert_eq!(ep, EncryptedPayload::default());
        assert!(ep.subtype().is_empty());
    }
}
