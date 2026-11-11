//! `Aspose.Pdf.Forms.DocMDPSignature` — a DocMDP (document
//! modification detection & prevention) signature, per
//! `capabilities/doc_mdp_signature.yaml`.

use crate::public::doc_mdp_access_permissions::DocMDPAccessPermissions;
use crate::public::signature::Signature;

/// A DocMDP signature restricting post-signing modifications.
///
/// `Eq` is not derived: this now holds the [`Signature`] the canonical
/// constructor takes, and that type is only `PartialEq` — its custom
/// appearance carries floating-point sizes.
#[derive(Debug, Clone, PartialEq)]
pub struct DocMDPSignature {
    access_permissions: DocMDPAccessPermissions,
    /// The signature this certification wraps. Crate-internal: the
    /// canonical constructor takes one, and what it carries — when the
    /// document was signed, how much room the PKCS#7 needs — is what
    /// the signing call reads.
    signature: Signature,
}

impl DocMDPSignature {
    /// Construct for the given signature with the given access
    /// permissions. The signature supplies context; the permission
    /// level is recorded.
    pub fn new(signature: &Signature, access_permissions: DocMDPAccessPermissions) -> Self {
        Self { access_permissions, signature: signature.clone() }
    }

    /// The wrapped signature. Crate-internal — read by the signing
    /// call for the entries the signature object carries.
    pub(crate) fn signature(&self) -> &Signature {
        &self.signature
    }

    /// The allowed post-signing modifications.
    pub fn access_permissions(&self) -> DocMDPAccessPermissions {
        self.access_permissions
    }
}
