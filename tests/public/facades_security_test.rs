// Public-API smoke tests for the security / signing / stamping facades —
// ported from the C++ sibling's facades_pdf_file_security,
// facades_pdf_file_signature, and facades_pdf_file_stamp(_draw) smoke tests.
//
// Scenarios assert only what the Rust facades actually do. PdfFileStamp
// draws its text into the bound page's content stream, so the stamped
// text does reach the saved file and is asserted at byte level below.

use aspose_pdf_foss::{
    Algorithm, DocMDPAccessPermissions, Document, DocumentPrivilege, KeySize, NumberingStyle,
    PdfFileSecurity, PdfFileSignature, PdfFileStamp, PdfFormat, SignatureName, PKCS7,
};
use std::path::{Path, PathBuf};

fn hello_world_pdf() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_extractor/hello_world.pdf")
        .to_string_lossy()
        .into_owned()
}

fn two_pages_pdf() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_extractor/two_pages.pdf")
        .to_string_lossy()
        .into_owned()
}

fn tmp(name: &str) -> String {
    common::unique_temp(format!("aspose_rust_facsec_{name}"))
        .to_string_lossy()
        .into_owned()
}

// =============================================================================
// PdfFileSecurity
// =============================================================================

#[test]
fn security_encrypt_file_then_reopen_with_password() {
    let out = tmp("enc_aes.pdf");
    let mut sec = PdfFileSecurity::with_files(hello_world_pdf(), out.clone());
    assert!(sec.encrypt_file_with_cipher(
        "user",
        "owner",
        &DocumentPrivilege::allow_all(),
        KeySize::x128,
        Algorithm::AES,
    ));
    assert!(Path::new(&out).exists());

    assert!(Document::open_with_password(&out, "user").unwrap().is_encrypted());
    assert!(Document::open_with_password(&out, "owner").unwrap().is_encrypted());
    assert!(Document::open_with_password(&out, "wrong").is_err());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn security_encrypt_file_default_cipher() {
    let out = tmp("enc_default.pdf");
    let mut sec = PdfFileSecurity::new();
    sec.set_input_file(hello_world_pdf());
    sec.set_output_file(out.clone());
    assert!(sec.encrypt_file("u", "o", &DocumentPrivilege::print(), KeySize::x128));
    assert!(Document::open_with_password(&out, "u").unwrap().is_encrypted());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn security_decrypt_file_round_trip() {
    let enc = tmp("dec_src.pdf");
    let dec = tmp("dec_out.pdf");

    let mut enc_sec = PdfFileSecurity::with_files(hello_world_pdf(), enc.clone());
    assert!(enc_sec.encrypt_file_with_cipher(
        "user",
        "owner",
        &DocumentPrivilege::allow_all(),
        KeySize::x128,
        Algorithm::AES,
    ));

    let mut dec_sec = PdfFileSecurity::with_files(enc.clone(), dec.clone());
    assert!(dec_sec.decrypt_file("owner"));
    assert!(Path::new(&dec).exists());

    // No password — the output is decrypted.
    assert!(!Document::open(&dec).unwrap().is_encrypted());
    let _ = std::fs::remove_file(&enc);
    let _ = std::fs::remove_file(&dec);
}

#[test]
fn security_change_password_round_trip() {
    let enc = tmp("chg_src.pdf");
    let out = tmp("chg_out.pdf");

    let mut enc_sec = PdfFileSecurity::with_files(hello_world_pdf(), enc.clone());
    assert!(enc_sec.encrypt_file_with_cipher(
        "user",
        "owner",
        &DocumentPrivilege::allow_all(),
        KeySize::x128,
        Algorithm::AES,
    ));

    let mut chg = PdfFileSecurity::with_files(enc.clone(), out.clone());
    assert!(chg.change_password("owner", "newuser", "newowner"));

    assert!(Document::open_with_password(&out, "newuser").unwrap().is_encrypted());
    assert!(Document::open_with_password(&out, "user").is_err());
    let _ = std::fs::remove_file(&enc);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn security_set_privilege_with_passwords() {
    // `set_privilege_with_passwords` re-encrypts (it decrypts the source
    // first), so it operates on an already-encrypted input. Encrypt, then
    // change the privilege under new passwords.
    let enc = tmp("setpriv_src.pdf");
    let out = tmp("setpriv.pdf");

    let mut enc_sec = PdfFileSecurity::with_files(hello_world_pdf(), enc.clone());
    assert!(enc_sec.encrypt_file_with_cipher(
        "user",
        "owner",
        &DocumentPrivilege::allow_all(),
        KeySize::x128,
        Algorithm::AES,
    ));

    let mut sec = PdfFileSecurity::with_files(enc.clone(), out.clone());
    assert!(sec.set_privilege_with_passwords("newuser", "owner", &DocumentPrivilege::print()));
    assert!(Document::open_with_password(&out, "newuser").unwrap().is_encrypted());
    let _ = std::fs::remove_file(&enc);
    let _ = std::fs::remove_file(&out);
}

#[test]
#[should_panic(expected = "password_recovery")]
fn security_set_privilege_without_passwords_is_pending() {
    // The single-argument overload needs the bound document's recovered
    // passwords; the (user, owner, privilege) overload is the wired path.
    let mut sec = PdfFileSecurity::with_files(hello_world_pdf(), tmp("setpriv_stub.pdf"));
    sec.set_privilege(&DocumentPrivilege::forbid_all());
}

#[test]
fn security_no_input_returns_false() {
    // No input bound: encrypt reports failure via the bool return rather
    // than raising (Rust always reports via the return value).
    let mut sec = PdfFileSecurity::new();
    sec.set_output_file(tmp("nope.pdf"));
    assert!(!sec.encrypt_file("u", "o", &DocumentPrivilege::allow_all(), KeySize::x128));
    assert!(!sec.try_encrypt_file("u", "o", &DocumentPrivilege::allow_all(), KeySize::x128));
}

#[test]
fn security_encrypt_aes256() {
    let out = tmp("enc_aes256.pdf");
    let mut sec = PdfFileSecurity::with_files(hello_world_pdf(), out.clone());
    assert!(sec.encrypt_file_with_cipher(
        "user",
        "owner",
        &DocumentPrivilege::allow_all(),
        KeySize::x256,
        Algorithm::AES,
    ));
    assert!(Document::open_with_password(&out, "user").unwrap().is_encrypted());
    let _ = std::fs::remove_file(&out);
}

// =============================================================================
// SignatureName (value type)
// =============================================================================

#[test]
fn signature_name_value_roundtrip() {
    let mut sn = SignatureName::new();
    assert!(sn.name.is_empty());
    assert!(sn.full_name.is_empty());
    assert!(!sn.has_signature());

    sn.name = "Signature1".into();
    sn.full_name = "form.Signature1".into();
    assert_eq!(sn.to_string(), "form.Signature1");

    let empty = SignatureName::new();
    assert_eq!(empty.to_string(), "");
}

// =============================================================================
// PdfFileSignature — unsigned-document read-back
// =============================================================================

#[test]
fn signature_enumerations_empty_on_unsigned_doc() {
    let sig = PdfFileSignature::with_document(Document::open(hello_world_pdf()).unwrap());
    assert!(!sig.contains_signature());
    assert!(!sig.is_contain_signature());
    assert!(!sig.contains_usage_rights());
    assert_eq!(sig.get_total_revision(), 0);
    assert!(sig.get_sign_names(false).is_empty());
    assert!(sig.get_signature_names(false).is_empty());
    assert!(sig.get_blank_sign_names().is_empty());
    assert!(sig.get_blank_signature_names().is_empty());
}

#[test]
fn signature_read_accessors_return_defaults() {
    let sig = PdfFileSignature::with_document(Document::open(hello_world_pdf()).unwrap());
    assert!(!sig.verify_signature("Sig1"));
    assert!(!sig.verify_signed("Sig1"));
    assert_eq!(sig.get_signer_name("Sig1"), "");
    assert_eq!(sig.get_reason("Sig1"), "");
    assert_eq!(sig.get_location("Sig1"), "");
    assert_eq!(sig.get_contact_info("Sig1"), "");
    assert_eq!(sig.get_revision("Sig1"), 0);
    assert!(!sig.covers_whole_document("Sig1"));
    assert_eq!(sig.get_access_permissions(), DocMDPAccessPermissions::NoChanges);
    assert!(!sig.is_ltv_enabled());
    assert!(!sig.is_certified());
}

#[test]
fn signature_certificate_and_appearance_round_trip() {
    let mut sig = PdfFileSignature::with_document(Document::open(hello_world_pdf()).unwrap());
    sig.set_certificate("cert.pfx", "pw");
    sig.set_signature_appearance("appearance.png");
    assert_eq!(sig.signature_appearance(), "appearance.png");
}

#[test]
#[should_panic(expected = "signature_revision_rewrite")]
fn signature_removal_is_pending() {
    // Removing a signature means rewriting the revision that carries it.
    let mut sig = PdfFileSignature::with_document(Document::open(hello_world_pdf()).unwrap());
    sig.remove_signature("Sig1");
}

#[test]
#[should_panic(expected = "signature_revision_rewrite")]
fn signature_usage_rights_removal_is_pending() {
    let mut sig = PdfFileSignature::with_document(Document::open(hello_world_pdf()).unwrap());
    sig.remove_usage_rights();
}

#[test]
fn signature_name_overloads() {
    let sig = PdfFileSignature::with_document(Document::open(hello_world_pdf()).unwrap());
    let mut sn = SignatureName::new();
    sn.full_name = "Sig1".into();

    assert!(!sig.verify_signature_by(&sn));
    assert_eq!(sig.get_signer_name_by(&sn), "");
    assert_eq!(sig.get_revision_by(&sn), 0);
    assert!(!sig.covers_whole_document_by(&sn));
}

// =============================================================================
// PdfFileSignature — real signing pipeline
// =============================================================================

#[test]
fn signature_sign_adds_readable_signature() {
    let out = tmp("signed_readable.pdf");
    {
        let mut sig = PdfFileSignature::with_document_and_output(
            Document::open(hello_world_pdf()).unwrap(),
            out.clone(),
        );
        sig.set_certificate("test.pfx", "pw");
        sig.sign(
            "Signature1",
            "I approve this document",
            "signer@example.com",
            "Prague",
            &PKCS7::new(),
        );
        sig.save().unwrap();
    }

    // Reload and read back through a fresh facade.
    let reader = PdfFileSignature::with_document(Document::open(&out).unwrap());
    assert!(reader.contains_signature());
    assert_eq!(reader.get_total_revision(), 1);

    let names = reader.get_sign_names(false);
    assert_eq!(names, vec!["Signature1".to_string()]);
    assert!(reader.get_blank_sign_names().is_empty());

    assert_eq!(reader.get_reason("Signature1"), "I approve this document");
    assert_eq!(reader.get_contact_info("Signature1"), "signer@example.com");
    assert_eq!(reader.get_location("Signature1"), "Prague");
    assert!(reader.covers_whole_document("Signature1"));
    assert!(reader.verify_signature("Signature1"));

    let sn = reader.get_signature_names(false);
    assert_eq!(sn.len(), 1);
    assert_eq!(sn[0].name, "Signature1");
    assert!(sn[0].has_signature());

    let _ = std::fs::remove_file(&out);
}

#[test]
fn signature_byte_range_and_contents_well_formed() {
    let out = tmp("signed_byterange.pdf");
    {
        let mut sig = PdfFileSignature::with_document_and_output(
            Document::open(hello_world_pdf()).unwrap(),
            out.clone(),
        );
        sig.sign_simple("Signature1", &PKCS7::new());
        sig.save().unwrap();
    }

    let bytes = std::fs::read(&out).unwrap();
    assert!(!bytes.is_empty());
    let s = String::from_utf8_lossy(&bytes);

    let br_at = s.find("/ByteRange [").unwrap() + "/ByteRange [".len();
    let br: Vec<usize> = s[br_at..]
        .split(']')
        .next()
        .unwrap()
        .split_whitespace()
        .map(|x| x.parse().unwrap())
        .collect();
    assert_eq!(br.len(), 4);
    assert_eq!(br[0], 0);
    // The gap [br1, br2) must wrap the /Contents hex string in '<' '>'.
    assert_eq!(bytes[br[1]], b'<');
    assert_eq!(bytes[br[2] - 1], b'>');
    // The byte ranges must cover the whole file.
    assert_eq!(br[2] + br[3], bytes.len());

    // /Contents hex → DER; SignedData ContentInfo starts with SEQUENCE (0x30).
    let lt = s.find("/Contents <").unwrap() + "/Contents <".len();
    let gt = s[lt..].find('>').unwrap() + lt;
    assert!(gt > lt);
    let first_hex = &s[lt..lt + 2];
    assert_eq!(u8::from_str_radix(first_hex, 16).unwrap(), 0x30);

    let _ = std::fs::remove_file(&out);
}

#[test]
fn signature_openssl_verifies_embedded_pkcs7() {
    use std::process::Command;
    if Command::new("sh")
        .arg("-c")
        .arg("command -v openssl")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        // openssl CLI not available — skip.
        return;
    }

    let out = tmp("signed_openssl.pdf");
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
    let br: Vec<usize> = s[br_at..]
        .split(']')
        .next()
        .unwrap()
        .split_whitespace()
        .map(|x| x.parse().unwrap())
        .collect();

    // Reconstruct the signed content (file minus the /Contents gap).
    let mut content = Vec::new();
    content.extend_from_slice(&bytes[..br[1]]);
    content.extend_from_slice(&bytes[br[2]..]);

    // Extract the /Contents hex → DER, trimming the trailing zero padding.
    let lt = s.find("/Contents <").unwrap() + "/Contents <".len();
    let gt = s[lt..].find('>').unwrap() + lt;
    let hexs = &s[lt..gt];
    let der: Vec<u8> = (0..hexs.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hexs[i..i + 2], 16).unwrap())
        .collect();
    let der_len = 2 + der_total_len(&der);
    let der = &der[..der_len.min(der.len())];

    let cpath: PathBuf = common::unique_temp("aspose_rust_facsec_content.bin");
    let dpath: PathBuf = common::unique_temp("aspose_rust_facsec_sig.der");
    std::fs::write(&cpath, &content).unwrap();
    std::fs::write(&dpath, der).unwrap();

    let cmd = format!(
        "openssl smime -verify -inform DER -in '{}' -content '{}' -noverify -out /dev/null 2>/dev/null",
        dpath.display(),
        cpath.display()
    );
    let ok = Command::new("sh").arg("-c").arg(&cmd).status().unwrap().success();

    let _ = std::fs::remove_file(&cpath);
    let _ = std::fs::remove_file(&dpath);
    let _ = std::fs::remove_file(&out);
    assert!(ok, "openssl failed to verify the embedded PKCS#7");
}

// Total DER length of a top-level SEQUENCE (tag + len header + content).
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

// =============================================================================
// PdfFileStamp
// =============================================================================

#[test]
fn stamp_position_constants() {
    assert_eq!(PdfFileStamp::POS_BOTTOM_MIDDLE, 0);
    assert_eq!(PdfFileStamp::POS_BOTTOM_RIGHT, 1);
    assert_eq!(PdfFileStamp::POS_UPPER_RIGHT, 2);
    assert_eq!(PdfFileStamp::POS_SIDES_RIGHT, 3);
    assert_eq!(PdfFileStamp::POS_UPPER_MIDDLE, 4);
    assert_eq!(PdfFileStamp::POS_BOTTOM_LEFT, 5);
    assert_eq!(PdfFileStamp::POS_SIDES_LEFT, 6);
    assert_eq!(PdfFileStamp::POS_UPPER_LEFT, 7);
}

#[test]
fn stamp_defaults() {
    let st = PdfFileStamp::new();
    assert!(!st.optimize_size());
    assert!(!st.keep_security());
    assert!(st.input_file().is_empty());
    assert!(st.output_file().is_empty());
    assert_eq!(st.page_number_rotation(), 0.0);
    assert_eq!(st.starting_number(), 1);
    assert_eq!(st.stamp_id(), 0);
    assert_eq!(st.page_height(), 792.0);
    assert_eq!(st.page_width(), 612.0);
    assert_eq!(st.numbering_style(), NumberingStyle::NumeralsArabic);
}

#[test]
fn stamp_property_roundtrip() {
    let mut st = PdfFileStamp::new();
    st.set_optimize_size(true);
    st.set_keep_security(true);
    st.set_input_file("in.pdf");
    st.set_output_file("out.pdf");
    st.set_page_number_rotation(90.0);
    st.set_starting_number(5);
    st.set_stamp_id(42);
    st.set_convert_to(PdfFormat::PDF_A_1B);

    assert!(st.optimize_size());
    assert!(st.keep_security());
    assert_eq!(st.input_file(), "in.pdf");
    assert_eq!(st.output_file(), "out.pdf");
    assert_eq!(st.page_number_rotation(), 90.0);
    assert_eq!(st.starting_number(), 5);
    assert_eq!(st.stamp_id(), 42);
}

#[test]
fn stamp_stubs_do_not_panic_and_save_passes_through() {
    use aspose_pdf_foss::ISaveableFacade;
    let mut st = PdfFileStamp::with_document(Document::open(hello_world_pdf()).unwrap());

    st.add_page_number("Page #");
    st.add_page_number_from("Page #", 1);
    st.add_page_number_at("Page #", 10.0, 20.0);
    st.add_page_number_in_rect("Page #", 1, 0.0, 0.0, 100.0, 20.0);
    st.add_header("Header", 36.0);
    st.add_header_with_margins("Header", 36.0, 36.0, 36.0);
    st.add_footer("Footer", 36.0);
    st.add_footer_with_margins("Footer", 36.0, 36.0, 36.0);

    let out = tmp("save.pdf");
    st.save(&out).unwrap();
    assert!(Path::new(&out).exists());
    let _ = std::fs::remove_file(&out);
}

#[test]
fn stamp_file_ctor_binds() {
    use aspose_pdf_foss::ISaveableFacade;
    let out = tmp("filector.pdf");
    let st = PdfFileStamp::with_files_and_security(two_pages_pdf(), &out, true).unwrap();
    assert_eq!(st.output_file(), out);
    assert!(st.keep_security());
    st.save(&out).unwrap();
    assert!(Path::new(&out).exists());
    let _ = std::fs::remove_file(&out);
}

fn doc_text(path: &str) -> String {
    use aspose_pdf_foss::TextAbsorber;
    let mut abs = TextAbsorber::new();
    abs.visit_document(&Document::open(path).unwrap()).unwrap();
    abs.text().to_string()
}

#[test]
fn stamp_save_renders_stamps_and_preserves_original_content() {
    use aspose_pdf_foss::ISaveableFacade;
    // The stamping methods draw into the bound pages; the original
    // content must also survive the save.
    let out = tmp("preserve.pdf");
    {
        let mut st = PdfFileStamp::with_document(Document::open(two_pages_pdf()).unwrap());
        st.add_header("CONFIDENTIAL", 36.0);
        st.add_footer("Draft v2", 36.0);
        st.add_page_number("Page #");
        st.save(&out).unwrap();
    }
    let text = doc_text(&out);
    assert!(text.contains("Page one"));
    assert!(text.contains("Page two"));
    assert!(text.contains("CONFIDENTIAL"));
    assert!(text.contains("Draft v2"));
    let _ = std::fs::remove_file(&out);
}

// ---- facades_pdf_file_stamp_draw_smoke_test.cpp ------------------

#[test]
fn stamp_header_appears_in_extracted_text() {
    use aspose_pdf_foss::ISaveableFacade;
    let out = tmp("stampdraw_header.pdf");
    {
        let mut st = PdfFileStamp::with_document(Document::open(two_pages_pdf()).unwrap());
        st.add_header("CONFIDENTIAL", 36.0);
        st.save(&out).unwrap();
    }
    assert!(doc_text(&out).contains("CONFIDENTIAL"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn stamp_footer_appears_in_extracted_text() {
    use aspose_pdf_foss::ISaveableFacade;
    let out = tmp("stampdraw_footer.pdf");
    {
        let mut st = PdfFileStamp::with_document(Document::open(two_pages_pdf()).unwrap());
        st.add_footer("Draft v2", 36.0);
        st.save(&out).unwrap();
    }
    assert!(doc_text(&out).contains("Draft v2"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn stamp_page_numbers_per_page() {
    use aspose_pdf_foss::ISaveableFacade;
    let out = tmp("stampdraw_pagenum.pdf");
    {
        let mut st = PdfFileStamp::with_document(Document::open(two_pages_pdf()).unwrap());
        st.add_page_number("Page #");
        st.save(&out).unwrap();
    }
    let text = doc_text(&out);
    assert!(text.contains("Page 1"), "got: {text:?}");
    assert!(text.contains("Page 2"), "got: {text:?}");
    assert!(text.contains("Page one")); // original content survives
    let _ = std::fs::remove_file(&out);
}

#[test]
fn stamp_starting_number_honored() {
    use aspose_pdf_foss::ISaveableFacade;
    let out = tmp("stampdraw_startnum.pdf");
    {
        let mut st = PdfFileStamp::with_document(Document::open(two_pages_pdf()).unwrap());
        st.add_page_number_from("#", 100);
        st.save(&out).unwrap();
    }
    let text = doc_text(&out);
    assert!(text.contains("100"), "got: {text:?}");
    assert!(text.contains("101"), "got: {text:?}");
    let _ = std::fs::remove_file(&out);
}

#[path = "../common/mod.rs"]
mod common;

/// A stamped page number is DRAWN, so this facade has to render the
/// number itself — a viewer formats a declared page *label*, not marks
/// on a page. The numbering style was stored and never consulted, so
/// every stamp came out in arabic digits whatever was asked for.
#[test]
fn a_stamped_number_follows_its_numbering_style() {
    use aspose_pdf_foss::ISaveableFacade;

    let stamped = |style: NumberingStyle| {
        let mut st = PdfFileStamp::with_document(Document::open(two_pages_pdf()).unwrap());
        st.set_numbering_style(style);
        st.add_page_number("- # -");
        let out = tmp("numbering.pdf");
        st.save(&out).unwrap();
        let bytes = std::fs::read(&out).unwrap();
        let _ = std::fs::remove_file(&out);
        String::from_utf8_lossy(&bytes).into_owned()
    };

    let arabic = stamped(NumberingStyle::NumeralsArabic);
    assert!(arabic.contains("(- 1 -)"), "page one in digits");
    assert!(arabic.contains("(- 2 -)"), "page two in digits");

    let roman = stamped(NumberingStyle::NumeralsRomanLowercase);
    assert!(roman.contains("(- i -)"), "page one in roman");
    assert!(roman.contains("(- ii -)"), "page two in roman");

    let letters = stamped(NumberingStyle::LettersUppercase);
    assert!(letters.contains("(- A -)"), "page one as a letter");
    assert!(letters.contains("(- B -)"), "page two as a letter");
}

/// The rotation was stored and never consulted either. A rotated run
/// needs the text matrix, since Td cannot turn anything (§9.4.2), and
/// the rotation belongs to the page number alone — a header stays
/// upright.
#[test]
fn a_stamped_number_can_be_turned() {
    use aspose_pdf_foss::ISaveableFacade;

    let mut st = PdfFileStamp::with_document(Document::open(hello_world_pdf()).unwrap());
    st.set_page_number_rotation(90.0);
    st.add_page_number("#");
    st.add_header("HEAD", 36.0);
    let out = tmp("rotation.pdf");
    st.save(&out).unwrap();
    let text = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
    let _ = std::fs::remove_file(&out);

    // 90° anticlockwise: cos 0, sin 1.
    assert!(text.contains("0 1 -1 0"), "the number's text matrix turns it");
    assert!(text.contains(" Tm"), "written as a text matrix, not an offset");
    assert!(text.contains("(HEAD)"), "the header is stamped too");
    assert_eq!(text.matches(" Tm").count(), 1, "but only the number turns");
}

/// Every signature this crate wrote carried the same hard-coded /M, a
/// date in June 2026, while the signature object's own date was
/// dropped. The date now comes from the object — and a signature given
/// no date carries no /M, rather than a time the caller never stated.
#[test]
fn a_signature_carries_the_date_it_was_given() {
    use aspose_pdf_foss::{DateTime, Signature};

    let signed = |cert: &Signature| {
        let out = tmp("sigdate.pdf");
        let mut sig = PdfFileSignature::with_document_and_output(
            Document::open(hello_world_pdf()).unwrap(),
            out.clone(),
        );
        sig.sign("sig", "why", "who", "where", cert);
        sig.save().unwrap();
        let bytes = std::fs::read(&out).unwrap();
        let _ = std::fs::remove_file(&out);
        String::from_utf8_lossy(&bytes).into_owned()
    };

    let mut dated = Signature::new();
    dated.set_date(DateTime::new(2026, 10, 2, 11, 30, 15));
    let text = signed(&dated);
    assert!(text.contains("/M (D:20261002113015Z)"), "the date given");
    assert!(text.contains("/Reason (why)"), "and the argument metadata");

    let undated = signed(&Signature::new());
    assert!(undated.contains("/Type /Sig"), "the signature is still written");
    assert!(!undated.contains("/M ("), "with no invented date");
}

/// The /Contents placeholder is sized from the signature object's
/// default length, which was dropped before. Asking for more room
/// makes a longer placeholder; asking for less cannot shrink it below
/// what this signer's own PKCS#7 needs.
#[test]
fn the_signature_placeholder_is_sized_from_the_signature() {
    use aspose_pdf_foss::Signature;

    let placeholder_len = |bytes: i32| {
        let mut cert = Signature::new();
        cert.set_default_signature_length(bytes);
        let out = tmp("siglen.pdf");
        let mut sig = PdfFileSignature::with_document_and_output(
            Document::open(hello_world_pdf()).unwrap(),
            out.clone(),
        );
        sig.sign("sig", "", "", "", &cert);
        sig.save().unwrap();
        let saved = std::fs::read(&out).unwrap();
        let _ = std::fs::remove_file(&out);
        let text = String::from_utf8_lossy(&saved).into_owned();
        let at = text.find("/Contents <").unwrap() + "/Contents <".len();
        text[at..].find('>').unwrap()
    };

    // The floor is 8 KiB of PKCS#7, so 16384 hex characters.
    assert_eq!(placeholder_len(3000), 16384, "the default asks for less than the floor");
    assert_eq!(placeholder_len(20000), 40000, "a bigger request is honoured");
}

/// A request below the floor still signs. Note what this does NOT
/// cover: a PKCS#7 too long for its placeholder used to be truncated,
/// leaving a /Contents that looks well-formed and fails to verify, and
/// the signer now refuses instead. Reaching that path needs a
/// certificate whose signature exceeds the floor, which the bundled one
/// cannot produce — so the guard is asserted by reading it, not here.
#[test]
fn a_tiny_length_request_is_raised_to_the_floor() {
    use aspose_pdf_foss::Signature;

    let mut cert = Signature::new();
    cert.set_default_signature_length(1);
    let out = tmp("sigfloor.pdf");
    let mut sig = PdfFileSignature::with_document_and_output(
        Document::open(hello_world_pdf()).unwrap(),
        out.clone(),
    );
    sig.sign("sig", "", "", "", &cert);
    sig.save().unwrap();
    let saved = std::fs::read(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    assert!(
        String::from_utf8_lossy(&saved).contains("/Type /Sig"),
        "the floor keeps the ordinary case signing"
    );
}
