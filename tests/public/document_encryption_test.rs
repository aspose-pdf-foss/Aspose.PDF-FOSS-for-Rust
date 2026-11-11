//! Public-surface tests for the `Document` decryption surface:
//! `open_with_password` / `is_encrypted` / `permissions`. Mirrors the
//! C++ `document_encrypted_open` + `document_permissions_getter` smoke
//! tests across the four classic + V5 cipher variants.

use aspose_pdf_foss::{CryptoAlgorithm, Document, Permissions};
use std::path::{Path, PathBuf};

fn enc_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/encryption").join(name)
}

fn two_pages() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/two_pages.pdf")
}

#[test]
fn unencrypted_doc_reports_false() {
    assert!(!Document::new().is_encrypted());
    assert!(!Document::open(two_pages()).unwrap().is_encrypted());
}

#[test]
fn v1_r2_rc40_user_and_owner() {
    for pw in ["user", "owner"] {
        let doc = Document::open_with_password(enc_fixture("v1_r2_rc40.pdf"), pw).unwrap();
        assert!(doc.is_encrypted(), "pw={pw}");
    }
}

#[test]
fn v1_r2_rc40_wrong_password_errors() {
    assert!(Document::open_with_password(enc_fixture("v1_r2_rc40.pdf"), "nope").is_err());
}

#[test]
fn v2_r3_rc128_user_and_wrong() {
    assert!(Document::open_with_password(enc_fixture("v2_r3_rc128.pdf"), "user").unwrap().is_encrypted());
    assert!(Document::open_with_password(enc_fixture("v2_r3_rc128.pdf"), "wrong").is_err());
}

#[test]
fn v4_r4_aes128_user_owner_and_wrong() {
    assert!(Document::open_with_password(enc_fixture("user_pwd_aes128.pdf"), "user").unwrap().is_encrypted());
    assert!(Document::open_with_password(enc_fixture("user_pwd_aes128.pdf"), "owner").unwrap().is_encrypted());
    assert!(Document::open_with_password(enc_fixture("user_pwd_aes128.pdf"), "bad").is_err());
}

#[test]
fn v5_r5_aes256_user_and_wrong() {
    assert!(Document::open_with_password(enc_fixture("v5_r5_aes256.pdf"), "user").unwrap().is_encrypted());
    assert!(Document::open_with_password(enc_fixture("v5_r5_aes256.pdf"), "nope").is_err());
}

#[test]
fn v5_r6_aes256_user_owner_and_wrong() {
    assert!(Document::open_with_password(enc_fixture("v5_r6_aes256.pdf"), "user").unwrap().is_encrypted());
    assert!(Document::open_with_password(enc_fixture("v5_r6_aes256.pdf"), "owner").unwrap().is_encrypted());
    assert!(Document::open_with_password(enc_fixture("v5_r6_aes256.pdf"), "wrong").is_err());
}

#[test]
fn plain_document_allows_everything() {
    let p = Document::open(two_pages()).unwrap().permissions();
    assert_ne!(p & Permissions::PRINT_DOCUMENT.bits(), 0);
    assert_ne!(p & Permissions::MODIFY_CONTENT.bits(), 0);
}

fn round_trip(algorithm: CryptoAlgorithm, tag: &str) {
    let out = common::unique_temp(format!("aspose_rust_encsave_{tag}.pdf"));
    {
        let mut doc = Document::new();
        doc.encrypt("user", "owner", Permissions::PRINT_DOCUMENT, algorithm);
        assert!(doc.is_encrypted(), "{tag}: staged");
        doc.save_to(&out).unwrap();
    }
    let reopened = Document::open_with_password(&out, "user").unwrap();
    assert!(reopened.is_encrypted(), "{tag}: reopen user");
    assert!(Document::open_with_password(&out, "owner").unwrap().is_encrypted(), "{tag}: reopen owner");
    assert!(Document::open_with_password(&out, "nope").is_err(), "{tag}: wrong pw");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn round_trip_rc4x40() {
    round_trip(CryptoAlgorithm::Rc4x40, "rc4x40");
}

#[test]
fn round_trip_rc4x128() {
    round_trip(CryptoAlgorithm::Rc4x128, "rc4x128");
}

#[test]
fn round_trip_aesx128() {
    round_trip(CryptoAlgorithm::Aesx128, "aesx128");
}

#[test]
fn round_trip_aesx256() {
    round_trip(CryptoAlgorithm::Aesx256, "aesx256");
}

#[test]
fn empty_owner_password_ok() {
    let out = common::unique_temp("aspose_rust_encsave_emptyowner.pdf");
    {
        let mut doc = Document::new();
        doc.encrypt("user", "", Permissions::PRINT_DOCUMENT, CryptoAlgorithm::Aesx128);
        doc.save_to(&out).unwrap();
    }
    assert!(Document::open_with_password(&out, "user").unwrap().is_encrypted());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn reflects_staged_encryption() {
    let mut doc = Document::open(two_pages()).unwrap();
    doc.encrypt("user", "owner", Permissions::PRINT_DOCUMENT, CryptoAlgorithm::Rc4x128);
    let p = doc.permissions();
    assert_ne!(p & Permissions::PRINT_DOCUMENT.bits(), 0, "print permitted");
    assert_eq!(p & Permissions::MODIFY_CONTENT.bits(), 0, "modify denied");
}

#[path = "../common/mod.rs"]
mod common;
