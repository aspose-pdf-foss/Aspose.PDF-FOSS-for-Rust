//! `Aspose.Pdf.Forms.PKCS7Detached` — a detached PKCS#7 (adbe.pkcs7.detached) signature, per
//! `capabilities/pkc_s7_detached.yaml`.
//!
//! Extends [`Signature`] (Rust models
//! the inheritance by embedding a base `Signature` + [`Deref`] so the
//! inherited metadata accessors apply). Carries the PFX certificate path +
//! password the signing pipeline pairs with the bundled test signer. The
//! `Stream` / `DigestHashAlgorithm` constructors are dropped.

use crate::public::signature::Signature;
use std::ops::{Deref, DerefMut};

/// A detached PKCS#7 (adbe.pkcs7.detached) signature configuration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PKCS7Detached {
    base: Signature,
    pfx_path: String,
    password: String,
}

impl PKCS7Detached {
    /// Construct an empty signature.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a PFX certificate file path and its password.
    pub fn with_certificate(pfx_path: impl Into<String>, password: impl Into<String>) -> Self {
        Self { base: Signature::new(), pfx_path: pfx_path.into(), password: password.into() }
    }

}

impl Deref for PKCS7Detached {
    type Target = Signature;
    fn deref(&self) -> &Signature {
        &self.base
    }
}

impl DerefMut for PKCS7Detached {
    fn deref_mut(&mut self) -> &mut Signature {
        &mut self.base
    }
}
