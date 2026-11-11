//! `Aspose.Pdf.Facades.PdfFileSignature` — the digital-signature facade,
//! per `capabilities/pdf_file_signature.yaml`.
//!
//! Extends [`SaveableFacade`](crate::SaveableFacade). `Sign` / `Certify`
//! perform real two-phase signing (a detached `adbe.pkcs7.detached`
//! PKCS#7 over the `/ByteRange`, via `document_sign`); the enumeration /
//! verify surface reads the AcroForm `/Sig` fields back. `RemoveSignature*`
//! / `RemoveUsageRights` are stubs (a revision rewrite is deferred). The
//! `Drawing.Rectangle` Sign/Certify overloads, the `DateTime` getters, the
//! `ValidationOptions` / `X509` / `SignatureAlgorithmInfo` verify
//! overloads, and the `Stream` extract surfaces are dropped.

use crate::internal::document_sign::{self, SignParams};
use crate::public::doc_mdp_access_permissions::DocMDPAccessPermissions;
use crate::public::doc_mdp_signature::DocMDPSignature;
use crate::public::document::Document;
use crate::public::error::Error;
use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::signature::Signature;
use crate::public::signature_name::SignatureName;
use std::path::Path;
use crate::internal::pending::PENDING;

/// A facade for digitally signing PDFs and reading signatures back.
#[derive(Debug, Default)]
pub struct PdfFileSignature {
    base: FacadeBase,
    input_file: String,
    output_file: String,
    pfx_path: String,
    password: String,
    signature_appearance: String,
    signed_bytes: Option<Vec<u8>>,
}

impl PdfFileSignature {
    /// Construct an unbound facade.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct bound to an input PDF file.
    pub fn with_file(input_file: impl Into<String>) -> Result<Self, Error> {
        let input_file = input_file.into();
        let document = Document::open(&input_file)?;
        Ok(Self { base: FacadeBase { document: Some(document) }, input_file, ..Self::default() })
    }

    /// Construct bound to an input PDF file, staging the output file.
    pub fn with_files(input_file: impl Into<String>, output_file: impl Into<String>) -> Result<Self, Error> {
        let input_file = input_file.into();
        let document = Document::open(&input_file)?;
        Ok(Self {
            base: FacadeBase { document: Some(document) },
            input_file,
            output_file: output_file.into(),
            ..Self::default()
        })
    }

    /// Construct bound to an in-memory [`Document`].
    pub fn with_document(document: Document) -> Self {
        Self { base: FacadeBase { document: Some(document) }, ..Self::default() }
    }

    /// Construct bound to an in-memory [`Document`], staging the output.
    pub fn with_document_and_output(document: Document, output_file: impl Into<String>) -> Self {
        Self {
            base: FacadeBase { document: Some(document) },
            output_file: output_file.into(),
            ..Self::default()
        }
    }

    /// The bytes currently representing the document (post-`Sign` if it
    /// has run, else the bound document's source).
    fn current_bytes(&self) -> Option<Vec<u8>> {
        if let Some(b) = &self.signed_bytes {
            return Some(b.clone());
        }
        self.base.document.as_ref().map(|d| d.internal_bytes().to_vec())
    }

    fn signatures(&self) -> Vec<document_sign::SigInfo> {
        match self.current_bytes() {
            Some(b) => document_sign::read_signatures(&b),
            None => Vec::new(),
        }
    }

    fn find<'a>(&self, sigs: &'a [document_sign::SigInfo], name: &str) -> Option<&'a document_sign::SigInfo> {
        sigs.iter().find(|s| s.name == name)
    }

    // ---- Signing ----

    /// Sign field `name` with the supplied metadata. Mirrors
    /// `Sign(sigName, reason, contact, location, Signature)`.
    pub fn sign(&mut self, name: &str, reason: &str, contact: &str, location: &str, cert: &Signature) {
        let Some(bytes) = self.current_bytes() else {
            return;
        };
        // The metadata comes from the arguments, as the canonical call
        // shape has it; the signature object carries the rest — when it
        // was signed and how much room its PKCS#7 needs.
        let params = SignParams {
            field_name: name.to_string(),
            reason: reason.to_string(),
            contact: contact.to_string(),
            location: location.to_string(),
            date: cert.date().to_pdf_date(),
            signature_bytes: cert.default_signature_length().max(0) as usize,
        };
        if let Some(signed) = document_sign::sign(&bytes, &params) {
            self.signed_bytes = Some(signed);
        }
    }

    /// Sign field `name` with empty metadata. Mirrors `Sign(sigName,
    /// Signature)`.
    pub fn sign_simple(&mut self, name: &str, cert: &Signature) {
        self.sign(name, "", "", "", cert);
    }

    /// Certify field `name`. Mirrors `Certify(sigName, DocMDPSignature)`.
    /// (A certification signature is produced like an approval signature
    /// in the shipped subset.)
    pub fn certify(&mut self, name: &str, signature: &DocMDPSignature) {
        let Some(bytes) = self.current_bytes() else {
            return;
        };
        let params = SignParams {
            field_name: name.to_string(),
            reason: String::new(),
            contact: String::new(),
            location: String::new(),
            date: signature.signature().date().to_pdf_date(),
            signature_bytes: signature.signature().default_signature_length().max(0) as usize,
        };
        if let Some(signed) = document_sign::sign(&bytes, &params) {
            self.signed_bytes = Some(signed);
        }
    }

    /// The document's certification access permissions.
    pub fn get_access_permissions(&self) -> DocMDPAccessPermissions {
        DocMDPAccessPermissions::NoChanges
    }

    // ---- Enumeration ----

    /// The names of the signed signature fields.
    pub fn get_sign_names(&self, _certified_only: bool) -> Vec<String> {
        self.signatures().into_iter().filter(|s| s.has_signature).map(|s| s.name).collect()
    }

    /// The signed signature fields as [`SignatureName`]s.
    pub fn get_signature_names(&self, _certified_only: bool) -> Vec<SignatureName> {
        self.signatures()
            .into_iter()
            .filter(|s| s.has_signature)
            .map(|s| SignatureName { name: s.name.clone(), full_name: s.name, has_signature: true })
            .collect()
    }

    /// The names of the blank (unsigned) signature fields.
    pub fn get_blank_sign_names(&self) -> Vec<String> {
        self.signatures().into_iter().filter(|s| !s.has_signature).map(|s| s.name).collect()
    }

    /// The blank signature fields as [`SignatureName`]s.
    pub fn get_blank_signature_names(&self) -> Vec<SignatureName> {
        self.signatures()
            .into_iter()
            .filter(|s| !s.has_signature)
            .map(|s| SignatureName { name: s.name.clone(), full_name: s.name, has_signature: false })
            .collect()
    }

    /// Whether the document contains any signature.
    pub fn contains_signature(&self) -> bool {
        self.signatures().iter().any(|s| s.has_signature)
    }

    /// Alias of [`contains_signature`](Self::contains_signature).
    pub fn is_contain_signature(&self) -> bool {
        self.contains_signature()
    }

    /// Whether the document declares usage rights.
    pub fn contains_usage_rights(&self) -> bool {
        false
    }

    /// The number of signature revisions.
    pub fn get_total_revision(&self) -> i32 {
        self.signatures().iter().filter(|s| s.has_signature).count() as i32
    }

    // ---- Read accessors ----

    /// Whether signature `name` covers the whole document.
    pub fn covers_whole_document(&self, name: &str) -> bool {
        self.find(&self.signatures(), name).map(|s| s.covers_whole).unwrap_or(false)
    }

    /// Alias of [`covers_whole_document`](Self::covers_whole_document).
    pub fn is_covers_whole_document(&self, name: &str) -> bool {
        self.covers_whole_document(name)
    }

    /// [`covers_whole_document`](Self::covers_whole_document) by
    /// [`SignatureName`].
    pub fn covers_whole_document_by(&self, name: &SignatureName) -> bool {
        self.covers_whole_document(&name.full_name)
    }

    /// The signer name of signature `name`.
    pub fn get_signer_name(&self, name: &str) -> String {
        self.find(&self.signatures(), name).map(|s| s.signer_name.clone()).unwrap_or_default()
    }
    /// [`get_signer_name`](Self::get_signer_name) by [`SignatureName`].
    pub fn get_signer_name_by(&self, name: &SignatureName) -> String {
        self.get_signer_name(&name.full_name)
    }

    /// The reason of signature `name`.
    pub fn get_reason(&self, name: &str) -> String {
        self.find(&self.signatures(), name).map(|s| s.reason.clone()).unwrap_or_default()
    }
    /// [`get_reason`](Self::get_reason) by [`SignatureName`].
    pub fn get_reason_by(&self, name: &SignatureName) -> String {
        self.get_reason(&name.full_name)
    }

    /// The location of signature `name`.
    pub fn get_location(&self, name: &str) -> String {
        self.find(&self.signatures(), name).map(|s| s.location.clone()).unwrap_or_default()
    }
    /// [`get_location`](Self::get_location) by [`SignatureName`].
    pub fn get_location_by(&self, name: &SignatureName) -> String {
        self.get_location(&name.full_name)
    }

    /// The contact info of signature `name`.
    pub fn get_contact_info(&self, name: &str) -> String {
        self.find(&self.signatures(), name).map(|s| s.contact.clone()).unwrap_or_default()
    }
    /// [`get_contact_info`](Self::get_contact_info) by [`SignatureName`].
    pub fn get_contact_info_by(&self, name: &SignatureName) -> String {
        self.get_contact_info(&name.full_name)
    }

    /// The signature revision of `name` (1 when signed, else 0).
    pub fn get_revision(&self, name: &str) -> i32 {
        if self.find(&self.signatures(), name).map(|s| s.has_signature).unwrap_or(false) {
            1
        } else {
            0
        }
    }
    /// [`get_revision`](Self::get_revision) by [`SignatureName`].
    pub fn get_revision_by(&self, name: &SignatureName) -> i32 {
        self.get_revision(&name.full_name)
    }

    /// Whether signature `name` is present + well-formed.
    pub fn verify_signature(&self, name: &str) -> bool {
        self.find(&self.signatures(), name).map(|s| s.has_signature && s.covers_whole).unwrap_or(false)
    }
    /// [`verify_signature`](Self::verify_signature) by [`SignatureName`].
    pub fn verify_signature_by(&self, name: &SignatureName) -> bool {
        self.verify_signature(&name.full_name)
    }
    /// Whether field `name` carries a signature.
    pub fn verify_signed(&self, name: &str) -> bool {
        self.find(&self.signatures(), name).map(|s| s.has_signature).unwrap_or(false)
    }

    // ---- Stubs / properties ----

    /// Whether LTV is enabled (always false in the shipped subset).
    pub fn is_ltv_enabled(&self) -> bool {
        false
    }
    /// Whether the document is certified (always false in the subset).
    pub fn is_certified(&self) -> bool {
        false
    }

    /// Remove a signature by name.
    ///
    /// Not implemented — pending `signature_revision_rewrite`. Calling
    /// this panics.
    pub fn remove_signature(&mut self, _name: &str) {
        PENDING("signature_revision_rewrite")
    }
    /// Remove a signature by name, optionally clearing the field.
    ///
    /// Not implemented — pending `signature_revision_rewrite`. Calling
    /// this panics.
    pub fn remove_signature_keep_field(&mut self, _name: &str, _remove_field: bool) {
        PENDING("signature_revision_rewrite")
    }
    /// Remove a signature by [`SignatureName`].
    ///
    /// Not implemented — pending `signature_revision_rewrite`. Calling
    /// this panics.
    pub fn remove_signature_by(&mut self, _name: &SignatureName) {
        PENDING("signature_revision_rewrite")
    }
    /// Remove a signature by [`SignatureName`], optionally clearing it.
    ///
    /// Not implemented — pending `signature_revision_rewrite`. Calling
    /// this panics.
    pub fn remove_signature_by_keep_field(&mut self, _name: &SignatureName, _remove_field: bool) {
        PENDING("signature_revision_rewrite")
    }
    /// Remove all signatures.
    ///
    /// Not implemented — pending `signature_revision_rewrite`. Calling
    /// this panics.
    pub fn remove_signatures(&mut self) {
        PENDING("signature_revision_rewrite")
    }
    /// Remove usage rights.
    ///
    /// Not implemented — pending `signature_revision_rewrite`. Calling
    /// this panics.
    pub fn remove_usage_rights(&mut self) {
        PENDING("signature_revision_rewrite")
    }

    /// Set the signing certificate (PFX path + password). The bundled
    /// test signer is used for the actual key operation.
    pub fn set_certificate(&mut self, pfx_path: impl Into<String>, password: impl Into<String>) {
        self.pfx_path = pfx_path.into();
        self.password = password.into();
    }

    /// The signature appearance image path.
    pub fn signature_appearance(&self) -> &str {
        &self.signature_appearance
    }
    /// Set the signature appearance image path.
    pub fn set_signature_appearance(&mut self, value: impl Into<String>) {
        self.signature_appearance = value.into();
    }

    /// The staged input file path.
    ///
    /// Carried, not read: the facade signs the document
    /// [`bind_pdf`](crate::IFacade::bind_pdf) opened, so this names a
    /// second input nothing consults.
    /// [`output_file`](PdfFileSignature::output_file) beside it *is*
    /// read — the no-arg `Save` writes there.
    pub fn input_file(&self) -> &str {
        &self.input_file
    }
    /// Set the input file path.
    pub fn set_input_file(&mut self, value: impl Into<String>) {
        self.input_file = value.into();
    }

    /// The staged output file path.
    pub fn output_file(&self) -> &str {
        &self.output_file
    }
    /// Set the output file path.
    pub fn set_output_file(&mut self, value: impl Into<String>) {
        self.output_file = value.into();
    }

    // ---- Save ----

    /// Save the (signed) document to `path`. Mirrors `Save(String)`.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let bytes = self.current_bytes().ok_or_else(|| Error::InvalidPdf("no document bound".into()))?;
        std::fs::write(path.as_ref(), bytes)?;
        Ok(())
    }

    /// Save the (signed) document to a byte sink. Mirrors `Save(Stream)`.
    pub fn save_to_writer(&self, mut writer: impl std::io::Write) -> Result<(), Error> {
        let bytes = self.current_bytes().ok_or_else(|| Error::InvalidPdf("no document bound".into()))?;
        writer.write_all(&bytes)?;
        Ok(())
    }

    /// Save the (signed) document to the staged output file. Mirrors the
    /// no-arg `Save()`.
    pub fn save(&self) -> Result<(), Error> {
        if self.output_file.is_empty() {
            return Err(Error::InvalidPdf("no output file set".into()));
        }
        self.save_to_file(&self.output_file)
    }
}

impl_facade!(PdfFileSignature);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::pkcs7::PKCS7;

    fn hello_world_pdf() -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_extractor/hello_world.pdf")
            .to_string_lossy()
            .into_owned()
    }

    fn tmp(name: &str) -> String {
        std::env::temp_dir().join(format!("aspose_rust_sig_{name}.pdf")).to_string_lossy().into_owned()
    }

    #[test]
    fn enumerations_empty_on_unsigned_doc() {
        let sig = PdfFileSignature::with_document(Document::open(hello_world_pdf()).unwrap());
        assert!(!sig.contains_signature());
        assert!(!sig.is_contain_signature());
        assert!(!sig.contains_usage_rights());
        assert_eq!(sig.get_total_revision(), 0);
        assert!(sig.get_sign_names(false).is_empty());
        assert!(sig.get_signature_names(false).is_empty());
        assert!(sig.get_blank_sign_names().is_empty());
    }

    #[test]
    fn read_accessors_return_defaults() {
        let sig = PdfFileSignature::with_document(Document::open(hello_world_pdf()).unwrap());
        assert!(!sig.verify_signature("Sig1"));
        assert!(!sig.verify_signed("Sig1"));
        assert_eq!(sig.get_signer_name("Sig1"), "");
        assert_eq!(sig.get_reason("Sig1"), "");
        assert_eq!(sig.get_revision("Sig1"), 0);
        assert!(!sig.covers_whole_document("Sig1"));
        assert_eq!(sig.get_access_permissions(), DocMDPAccessPermissions::NoChanges);
        assert!(!sig.is_ltv_enabled());
        assert!(!sig.is_certified());
    }

    #[test]
    fn sign_adds_readable_signature() {
        let out = tmp("readable");
        {
            let mut sig = PdfFileSignature::with_document_and_output(
                Document::open(hello_world_pdf()).unwrap(),
                out.clone(),
            );
            sig.sign("Signature1", "I approve this document", "signer@example.com", "Prague", &PKCS7::new());
            sig.save().unwrap();
        }
        let reader = PdfFileSignature::with_document(Document::open(&out).unwrap());
        assert!(reader.contains_signature());
        assert_eq!(reader.get_total_revision(), 1);
        let names = reader.get_sign_names(false);
        assert_eq!(names, vec!["Signature1".to_string()]);
        assert!(reader.get_blank_sign_names().is_empty());
        assert_eq!(reader.get_signer_name("Signature1"), document_sign::SIGNER_NAME);
        assert_eq!(reader.get_reason("Signature1"), "I approve this document");
        assert_eq!(reader.get_contact_info("Signature1"), "signer@example.com");
        assert_eq!(reader.get_location("Signature1"), "Prague");
        assert!(reader.covers_whole_document("Signature1"));
        assert!(reader.verify_signature("Signature1"));
        let sn = reader.get_signature_names(false);
        assert_eq!(sn.len(), 1);
        assert!(sn[0].has_signature());
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn byte_range_and_contents_well_formed() {
        let out = tmp("byterange");
        {
            let mut sig = PdfFileSignature::with_document_and_output(
                Document::open(hello_world_pdf()).unwrap(),
                out.clone(),
            );
            sig.sign_simple("Signature1", &PKCS7::new());
            sig.save().unwrap();
        }
        let bytes = std::fs::read(&out).unwrap();
        let s = String::from_utf8_lossy(&bytes);
        let br_at = s.find("/ByteRange [").unwrap() + "/ByteRange [".len();
        let br: Vec<usize> = s[br_at..].split(']').next().unwrap().split_whitespace().map(|x| x.parse().unwrap()).collect();
        assert_eq!(br.len(), 4);
        assert_eq!(br[0], 0);
        assert_eq!(bytes[br[1]], b'<');
        assert_eq!(bytes[br[2] - 1], b'>');
        assert_eq!(br[2] + br[3], bytes.len());
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn openssl_verifies_embedded_pkcs7() {
        use std::process::Command;
        if Command::new("sh").arg("-c").arg("command -v openssl").output().map(|o| !o.status.success()).unwrap_or(true) {
            return;
        }
        let out = tmp("openssl");
        {
            let mut sig = PdfFileSignature::with_document_and_output(
                Document::open(hello_world_pdf()).unwrap(),
                out.clone(),
            );
            sig.sign("Signature1", "openssl-verify", "x@y.z", "Earth", &PKCS7::new());
            sig.save().unwrap();
        }
        let bytes = std::fs::read(&out).unwrap();
        let s = String::from_utf8_lossy(&bytes);
        let br_at = s.find("/ByteRange [").unwrap() + "/ByteRange [".len();
        let br: Vec<usize> = s[br_at..].split(']').next().unwrap().split_whitespace().map(|x| x.parse().unwrap()).collect();

        let mut content = Vec::new();
        content.extend_from_slice(&bytes[..br[1]]);
        content.extend_from_slice(&bytes[br[2]..]);
        // Extract the /Contents hex → DER.
        let lt = s.find("/Contents <").unwrap() + "/Contents <".len();
        let gt = s[lt..].find('>').unwrap() + lt;
        let hexs: String = s[lt..gt].chars().collect();
        let der: Vec<u8> = (0..hexs.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hexs[i..i + 2], 16).unwrap())
            .collect();
        // Trim trailing zero padding back to the DER length.
        let der_len = 2 + der_total_len(&der);
        let der = &der[..der_len.min(der.len())];

        let dir = std::env::temp_dir();
        let cpath = dir.join("aspose_sig_content.bin");
        let dpath = dir.join("aspose_sig.der");
        std::fs::write(&cpath, &content).unwrap();
        std::fs::write(&dpath, der).unwrap();
        let cmd = format!(
            "openssl smime -verify -inform DER -in '{}' -content '{}' -noverify -out /dev/null 2>/dev/null",
            dpath.display(), cpath.display()
        );
        let ok = Command::new("sh").arg("-c").arg(&cmd).status().unwrap().success();
        let _ = std::fs::remove_file(&cpath);
        let _ = std::fs::remove_file(&dpath);
        let _ = std::fs::remove_file(&out);
        assert!(ok, "openssl failed to verify the embedded PKCS#7");
    }

    // Compute the total DER length of a top-level SEQUENCE (tag + len header + content).
    fn der_total_len(der: &[u8]) -> usize {
        if der.len() < 2 || der[0] != 0x30 {
            return der.len();
        }
        let l0 = der[1];
        if l0 < 0x80 {
            l0 as usize
        } else {
            let nb = (l0 & 0x7f) as usize;
            let mut len = 0usize;
            for i in 0..nb {
                len = (len << 8) | der[2 + i] as usize;
            }
            len + nb
        }
    }
}
