// Public-API smoke driver — FormPresentationMode (Phase 4 Beat 13).
//
// Two-variant enum; tests confirm the ordinal mapping matches
// the canonical Aspose.PDF 26.4.0 surface (Production=0, Editor=1)
// and that Default → Production (the canonical engine default).

use aspose_pdf_foss::FormPresentationMode;

#[test]
fn variant_ordinals_match_canonical() {
    assert_eq!(FormPresentationMode::Production as i32, 0);
    assert_eq!(FormPresentationMode::Editor as i32, 1);
}

#[test]
fn default_is_production() {
    assert_eq!(FormPresentationMode::default(), FormPresentationMode::Production);
}

#[test]
fn variants_round_trip_through_copy_clone() {
    let a = FormPresentationMode::Editor;
    let b = a; // Copy
    let c = a;
    assert_eq!(b, FormPresentationMode::Editor);
    assert_eq!(c, FormPresentationMode::Editor);
}

#[test]
fn variants_compare_with_eq() {
    assert_eq!(FormPresentationMode::Production, FormPresentationMode::Production);
    assert_ne!(FormPresentationMode::Production, FormPresentationMode::Editor);
}
