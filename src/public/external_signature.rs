//! `Aspose.Pdf.Forms.ExternalSignature` — a signature whose private-key
//! operation is performed externally, per `capabilities/external_signature.yaml`.
//!
//! Extends [`Signature`] (Rust embeds a
//! base `Signature` + [`Deref`]). The `X509Certificate2` constructors +
//! `Certificate` property and the `DigestHashAlgorithm` constructor are
//! dropped (those crypto types are outside the shipped subset); the PFX-path
//! constructor ships.

use crate::public::signature::Signature;
use std::ops::{Deref, DerefMut};

/// An external-signing signature configuration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExternalSignature {
    base: Signature,
    pfx_path: String,
    use_legacy_appearance: bool,
}

impl ExternalSignature {
    /// Construct from a PFX certificate file path. `use_legacy_appearance`
    /// mirrors the canonical `(string, bool)` constructor flag.
    pub fn with_certificate(pfx_path: impl Into<String>, use_legacy_appearance: bool) -> Self {
        Self {
            base: Signature::new(),
            pfx_path: pfx_path.into(),
            use_legacy_appearance,
        }
    }


    /// Whether the legacy appearance flag was set.
    pub fn use_legacy_appearance(&self) -> bool {
        self.use_legacy_appearance
    }
}

impl Deref for ExternalSignature {
    type Target = Signature;
    fn deref(&self) -> &Signature {
        &self.base
    }
}

impl DerefMut for ExternalSignature {
    fn deref_mut(&mut self) -> &mut Signature {
        &mut self.base
    }
}
