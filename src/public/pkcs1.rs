//! `Aspose.Pdf.Forms.PKCS1` — a PKCS#1 (adbe.x509.rsa_sha1) signature, per
//! `capabilities/pkc_s1.yaml`.
//!
//! Extends [`Signature`] (Rust models
//! the inheritance by embedding a base `Signature` + [`Deref`] so the
//! inherited metadata accessors apply). Carries the PFX certificate path +
//! password the signing pipeline pairs with the bundled test signer. The
//! `Stream` / `DigestHashAlgorithm` constructors are dropped.

use crate::public::signature::Signature;
use std::ops::{Deref, DerefMut};

/// A PKCS#1 (adbe.x509.rsa_sha1) signature configuration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PKCS1 {
    base: Signature,
    pfx_path: String,
    password: String,
}

impl PKCS1 {
    /// Construct an empty signature.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a PFX certificate file path and its password.
    pub fn with_certificate(pfx_path: impl Into<String>, password: impl Into<String>) -> Self {
        Self { base: Signature::new(), pfx_path: pfx_path.into(), password: password.into() }
    }

}

impl Deref for PKCS1 {
    type Target = Signature;
    fn deref(&self) -> &Signature {
        &self.base
    }
}

impl DerefMut for PKCS1 {
    fn deref_mut(&mut self) -> &mut Signature {
        &mut self.base
    }
}
