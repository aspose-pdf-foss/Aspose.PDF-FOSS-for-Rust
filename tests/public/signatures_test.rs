// Public-API smoke driver — F-6 signature surface.
//
// Signature (metadata + appearance) + SignatureCustomAppearance +
// DocMDPSignature + SignatureField, plus the three configuration
// subtypes that deref to Signature — PKCS1, PKCS7Detached and
// ExternalSignature. Cryptographic signing is deferred (the surface is
// exercised here).

use aspose_pdf_foss::{
    Annotation, AnnotationType, Color, DocMDPAccessPermissions, DocMDPSignature, Document,
    ExternalSignature, Field, Rectangle, Rotation, Signature, SignatureCustomAppearance,
    SignatureField, SubjectNameElements, PKCS1, PKCS7Detached,
};
use std::path::PathBuf;

fn page_doc() -> Document {
    Document::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("pages_tree/single.pdf"),
    )
    .unwrap()
}

#[test]
fn signature_metadata() {
    let mut s = Signature::new();
    assert!(!s.verify()); // deferred crypto verification
    assert!(s.show_properties());
    assert_eq!(s.default_signature_length(), 3000);
    assert!(s.byte_range().is_empty());

    s.set_reason("approval".into());
    s.set_location("HQ".into());
    s.set_contact_info("ada@x".into());
    s.set_use_ltv(true);
    assert_eq!(s.reason(), "approval");
    assert_eq!(s.location(), "HQ");
    assert_eq!(s.contact_info(), "ada@x");
    assert!(s.use_ltv());

    let _ = Signature::with_certificate("/tmp/cert.pfx".into(), "pw".into());
}

#[test]
fn custom_appearance() {
    let mut a = SignatureCustomAppearance::new();
    assert_eq!(a.rotation(), Rotation::None);
    assert!(!a.show_reason());
    a.set_show_reason(true);
    a.set_reason_label("Reason:".into());
    a.set_font_family_name("Helvetica".into());
    a.set_font_size(9.0);
    a.set_foreground_color(Color::blue());
    a.set_rotation(Rotation::On90);
    a.set_digital_subject_format(vec![SubjectNameElements::CN, SubjectNameElements::O]);
    assert!(a.show_reason());
    assert_eq!(a.reason_label(), "Reason:");
    assert_eq!(a.font_family_name(), "Helvetica");
    assert_eq!(a.font_size(), 9.0);
    assert_eq!(a.rotation(), Rotation::On90);
    assert_eq!(a.digital_subject_format().len(), 2);

    // a signature carries the appearance
    let mut s = Signature::new();
    s.set_custom_appearance(a);
    assert_eq!(s.custom_appearance().reason_label(), "Reason:");
}

#[test]
fn doc_mdp_signature() {
    let s = Signature::new();
    let mdp = DocMDPSignature::new(&s, DocMDPAccessPermissions::FillingInForms);
    assert_eq!(
        mdp.access_permissions(),
        DocMDPAccessPermissions::FillingInForms
    );
}

#[test]
fn signature_field() {
    let doc = page_doc();
    let page = doc.pages().get(1).unwrap();
    let mut f = SignatureField::on_page(&page, Rectangle::new(0.0, 0.0, 80.0, 30.0, false));
    assert_eq!(f.annotation_type(), AnnotationType::Widget);
    assert!(f.signature().is_none());
    f.set_partial_name("sig".into()); // inherited Field property

    let mut s = Signature::new();
    s.set_reason("sign-off".into());
    f.sign(s);
    assert!(f.signature().is_some());
    assert_eq!(f.signature().unwrap().reason(), "sign-off");
    assert_eq!(f.partial_name(), "sig");
}

#[test]
fn pkcs1_defaults_and_certificate_ctor() {
    // The subtypes carry a Signature base reachable through Deref, so the
    // inherited metadata surface works on them directly.
    let empty = PKCS1::new();
    assert_eq!(empty.reason(), "");
    assert_eq!(empty, PKCS1::default());

    let mut signed = PKCS1::with_certificate("cert.pfx", "pw");
    signed.set_reason("approval".into());
    signed.set_location("Prague".into());
    assert_eq!(signed.reason(), "approval");
    assert_eq!(signed.location(), "Prague");
    assert_ne!(signed, empty);
}

#[test]
fn pkcs7_detached_defaults_and_certificate_ctor() {
    let empty = PKCS7Detached::new();
    assert_eq!(empty.authority(), "");
    assert_eq!(empty, PKCS7Detached::default());

    let mut signed = PKCS7Detached::with_certificate("cert.pfx", "pw");
    signed.set_authority("CA".into());
    assert_eq!(signed.authority(), "CA");
    assert_ne!(signed, empty);
}

#[test]
fn external_signature_carries_the_legacy_appearance_flag() {
    let legacy = ExternalSignature::with_certificate("cert.pfx", true);
    assert!(legacy.use_legacy_appearance());

    let mut modern = ExternalSignature::with_certificate("cert.pfx", false);
    assert!(!modern.use_legacy_appearance());

    // Inherited Signature metadata is settable through the deref.
    modern.set_reason("external".into());
    assert_eq!(modern.reason(), "external");
    assert_ne!(modern, legacy);
}
