//! `Aspose.Pdf.Facades.PdfFileSecurity` — a facade for file-to-file
//! encryption, decryption, password change, and privilege change, per
//! `capabilities/pdf_file_security.yaml`.
//!
//! Extends [`SaveableFacade`](crate::SaveableFacade). The operations
//! read the staged [`input_file`](PdfFileSecurity::input_file) (or the
//! bound document), apply the security change through the real
//! [`Document`] encryption surface + `document_encrypt`, and write
//! [`output_file`](PdfFileSecurity::output_file).
//!
//! The `Stream` constructors / `InputStream` / `OutputStream` and
//! `LastException` (`System.Exception` has no idiomatic Rust surface —
//! the operations report success via the `bool` return) are dropped.

use crate::internal::document_encrypt;
use crate::public::algorithm::Algorithm;
use crate::public::crypto_algorithm::CryptoAlgorithm;
use crate::public::document::Document;
use crate::public::document_privilege::DocumentPrivilege;
use crate::public::i_facade::{impl_facade, FacadeBase};
use crate::public::key_size::KeySize;
use crate::internal::pending::PENDING;

/// A facade for document-security file operations.
#[derive(Debug, Default)]
pub struct PdfFileSecurity {
    base: FacadeBase,
    input_file: String,
    output_file: String,
    allow_exceptions: bool,
}

fn to_crypto(key_size: KeySize, algorithm: Algorithm) -> CryptoAlgorithm {
    match (algorithm, key_size) {
        (Algorithm::RC4, KeySize::x40) => CryptoAlgorithm::Rc4x40,
        (Algorithm::RC4, _) => CryptoAlgorithm::Rc4x128,
        (Algorithm::AES, KeySize::x256) => CryptoAlgorithm::Aesx256,
        (Algorithm::AES, _) => CryptoAlgorithm::Aesx128,
    }
}

impl PdfFileSecurity {
    /// Construct an unbound facade.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct with the input + output file paths staged.
    pub fn with_files(input_file: impl Into<String>, output_file: impl Into<String>) -> Self {
        Self {
            input_file: input_file.into(),
            output_file: output_file.into(),
            ..Self::default()
        }
    }

    /// Bind to an in-memory [`Document`].
    pub fn with_document(document: Document) -> Self {
        Self { base: FacadeBase { document: Some(document) }, ..Self::default() }
    }

    /// Bind to an in-memory [`Document`] and stage the output file.
    pub fn with_document_and_output(document: Document, output_file: impl Into<String>) -> Self {
        Self {
            base: FacadeBase { document: Some(document) },
            output_file: output_file.into(),
            ..Self::default()
        }
    }

    fn source_bytes(&self) -> Option<Vec<u8>> {
        if !self.input_file.is_empty() {
            std::fs::read(&self.input_file).ok()
        } else {
            self.base.document.as_ref().map(|d| d.internal_bytes().to_vec())
        }
    }

    fn write_output(&self, bytes: &[u8]) -> bool {
        !self.output_file.is_empty() && std::fs::write(&self.output_file, bytes).is_ok()
    }

    // ---- Encrypt ----

    /// Encrypt the input file with the default cipher (RC4) at `key_size`.
    pub fn encrypt_file(
        &mut self,
        user_password: &str,
        owner_password: &str,
        privilege: &DocumentPrivilege,
        key_size: KeySize,
    ) -> bool {
        self.encrypt_file_with_cipher(user_password, owner_password, privilege, key_size, Algorithm::RC4)
    }

    /// Encrypt the input file with an explicit `cipher` at `key_size`.
    pub fn encrypt_file_with_cipher(
        &mut self,
        user_password: &str,
        owner_password: &str,
        privilege: &DocumentPrivilege,
        key_size: KeySize,
        cipher: Algorithm,
    ) -> bool {
        let Some(bytes) = self.source_bytes() else {
            return false;
        };
        let Ok(mut doc) = Document::from_bytes(bytes) else {
            return false;
        };
        doc.encrypt_with_privilege(user_password, owner_password, privilege, to_crypto(key_size, cipher), true);
        !self.output_file.is_empty() && doc.save_to(&self.output_file).is_ok()
    }

    /// `TryEncryptFile` — same as [`encrypt_file`](Self::encrypt_file).
    pub fn try_encrypt_file(
        &mut self,
        user_password: &str,
        owner_password: &str,
        privilege: &DocumentPrivilege,
        key_size: KeySize,
    ) -> bool {
        self.encrypt_file(user_password, owner_password, privilege, key_size)
    }

    // ---- Decrypt ----

    /// Decrypt the input file with `owner_password`, writing a plaintext
    /// PDF to the output file.
    pub fn decrypt_file(&mut self, owner_password: &str) -> bool {
        let Some(bytes) = self.source_bytes() else {
            return false;
        };
        match document_encrypt::decrypt(&bytes, owner_password.as_bytes()) {
            Some(plain) => self.write_output(&plain),
            None => false,
        }
    }

    /// `TryDecryptFile` — same as [`decrypt_file`](Self::decrypt_file).
    pub fn try_decrypt_file(&mut self, owner_password: &str) -> bool {
        self.decrypt_file(owner_password)
    }

    // ---- Privilege ----

    /// `SetPrivilege(privilege)`` overload).
    ///
    /// Not implemented — pending `password_recovery`. Calling this
    /// panics.
    pub fn set_privilege(&mut self, _privilege: &DocumentPrivilege) -> bool {
        PENDING("password_recovery")
    }

    /// Re-encrypt with a new `privilege`, decrypting with `owner_password`
    /// and re-encrypting under `user_password` / `owner_password`.
    pub fn set_privilege_with_passwords(
        &mut self,
        user_password: &str,
        owner_password: &str,
        privilege: &DocumentPrivilege,
    ) -> bool {
        self.reencrypt(owner_password, user_password, owner_password, privilege, KeySize::x128, Algorithm::AES)
    }

    /// `TrySetPrivilege` — same as
    /// [`set_privilege_with_passwords`](Self::set_privilege_with_passwords).
    pub fn try_set_privilege(
        &mut self,
        user_password: &str,
        owner_password: &str,
        privilege: &DocumentPrivilege,
    ) -> bool {
        self.set_privilege_with_passwords(user_password, owner_password, privilege)
    }

    // ---- Change password ----

    /// Change the document passwords, authenticating with
    /// `owner_password` and re-encrypting under the new passwords.
    pub fn change_password(
        &mut self,
        owner_password: &str,
        new_user_password: &str,
        new_owner_password: &str,
    ) -> bool {
        self.reencrypt(
            owner_password,
            new_user_password,
            new_owner_password,
            &DocumentPrivilege::allow_all(),
            KeySize::x128,
            Algorithm::AES,
        )
    }

    /// `ChangePassword` with an explicit privilege + key size.
    pub fn change_password_with_privilege(
        &mut self,
        owner_password: &str,
        new_user_password: &str,
        new_owner_password: &str,
        privilege: &DocumentPrivilege,
        key_size: KeySize,
    ) -> bool {
        self.reencrypt(owner_password, new_user_password, new_owner_password, privilege, key_size, Algorithm::RC4)
    }

    /// `ChangePassword` with an explicit privilege, key size, and cipher.
    pub fn change_password_with_cipher(
        &mut self,
        owner_password: &str,
        new_user_password: &str,
        new_owner_password: &str,
        privilege: &DocumentPrivilege,
        key_size: KeySize,
        cipher: Algorithm,
    ) -> bool {
        self.reencrypt(owner_password, new_user_password, new_owner_password, privilege, key_size, cipher)
    }

    /// `TryChangePassword` — same as [`change_password`](Self::change_password).
    pub fn try_change_password(
        &mut self,
        owner_password: &str,
        new_user_password: &str,
        new_owner_password: &str,
    ) -> bool {
        self.change_password(owner_password, new_user_password, new_owner_password)
    }

    /// `TryChangePassword` with privilege + key size.
    pub fn try_change_password_with_privilege(
        &mut self,
        owner_password: &str,
        new_user_password: &str,
        new_owner_password: &str,
        privilege: &DocumentPrivilege,
        key_size: KeySize,
    ) -> bool {
        self.change_password_with_privilege(owner_password, new_user_password, new_owner_password, privilege, key_size)
    }

    /// `TryChangePassword` with privilege, key size, and cipher.
    pub fn try_change_password_with_cipher(
        &mut self,
        owner_password: &str,
        new_user_password: &str,
        new_owner_password: &str,
        privilege: &DocumentPrivilege,
        key_size: KeySize,
        cipher: Algorithm,
    ) -> bool {
        self.change_password_with_cipher(owner_password, new_user_password, new_owner_password, privilege, key_size, cipher)
    }

    /// Decrypt with `owner_password`, then re-encrypt under the new
    /// passwords / privilege / cipher and write the output file.
    fn reencrypt(
        &mut self,
        owner_password: &str,
        new_user: &str,
        new_owner: &str,
        privilege: &DocumentPrivilege,
        key_size: KeySize,
        cipher: Algorithm,
    ) -> bool {
        let Some(bytes) = self.source_bytes() else {
            return false;
        };
        let Some(plain) = document_encrypt::decrypt(&bytes, owner_password.as_bytes()) else {
            return false;
        };
        let Ok(mut doc) = Document::from_bytes(plain) else {
            return false;
        };
        doc.encrypt_with_privilege(new_user, new_owner, privilege, to_crypto(key_size, cipher), true);
        !self.output_file.is_empty() && doc.save_to(&self.output_file).is_ok()
    }

    // ---- Properties ----

    /// The staged input file path.
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

    /// Whether operations raise on error (vs reporting via the `bool`
    /// return). Rust always reports via the return; the flag is staged
    /// for API parity.
    pub fn allow_exceptions(&self) -> bool {
        self.allow_exceptions
    }
    /// Set the `allow_exceptions` flag.
    pub fn set_allow_exceptions(&mut self, value: bool) {
        self.allow_exceptions = value;
    }
}

impl_facade!(PdfFileSecurity);

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn hello_world_pdf() -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/text_extractor/hello_world.pdf")
            .to_string_lossy()
            .into_owned()
    }

    fn tmp(name: &str) -> String {
        std::env::temp_dir().join(format!("aspose_rust_filesec_{name}.pdf")).to_string_lossy().into_owned()
    }

    #[test]
    fn encrypt_file_then_reopen_with_password() {
        let out = tmp("enc_aes");
        let mut sec = PdfFileSecurity::with_files(hello_world_pdf(), out.clone());
        assert!(sec.encrypt_file_with_cipher("user", "owner", &DocumentPrivilege::allow_all(), KeySize::x128, Algorithm::AES));
        assert!(Path::new(&out).exists());

        assert!(Document::open_with_password(&out, "user").unwrap().is_encrypted());
        assert!(Document::open_with_password(&out, "owner").unwrap().is_encrypted());
        assert!(Document::open_with_password(&out, "wrong").is_err());
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn encrypt_file_default_cipher() {
        let out = tmp("enc_default");
        let mut sec = PdfFileSecurity::new();
        sec.set_input_file(hello_world_pdf());
        sec.set_output_file(out.clone());
        assert!(sec.encrypt_file("u", "o", &DocumentPrivilege::print(), KeySize::x128));
        assert!(Document::open_with_password(&out, "u").unwrap().is_encrypted());
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn decrypt_file_round_trip() {
        let enc = tmp("dec_src");
        let dec = tmp("dec_out");
        let mut enc_sec = PdfFileSecurity::with_files(hello_world_pdf(), enc.clone());
        assert!(enc_sec.encrypt_file_with_cipher("user", "owner", &DocumentPrivilege::allow_all(), KeySize::x128, Algorithm::AES));

        let mut dec_sec = PdfFileSecurity::with_files(enc.clone(), dec.clone());
        assert!(dec_sec.decrypt_file("owner"));
        assert!(Path::new(&dec).exists());

        // No password — the output is decrypted.
        assert!(!Document::open(&dec).unwrap().is_encrypted());
        let _ = std::fs::remove_file(&enc);
        let _ = std::fs::remove_file(&dec);
    }

    #[test]
    fn change_password_round_trip() {
        let enc = tmp("chg_src");
        let out = tmp("chg_out");
        let mut enc_sec = PdfFileSecurity::with_files(hello_world_pdf(), enc.clone());
        assert!(enc_sec.encrypt_file_with_cipher("user", "owner", &DocumentPrivilege::allow_all(), KeySize::x128, Algorithm::AES));

        let mut chg = PdfFileSecurity::with_files(enc.clone(), out.clone());
        assert!(chg.change_password("owner", "newuser", "newowner"));

        assert!(Document::open_with_password(&out, "newuser").unwrap().is_encrypted());
        assert!(Document::open_with_password(&out, "user").is_err());
        let _ = std::fs::remove_file(&enc);
        let _ = std::fs::remove_file(&out);
    }
}
