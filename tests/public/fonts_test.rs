//! Text cluster T-1: Font + FontRepository public surface.

use aspose_pdf_foss::{Error, Font, FontRepository};
use std::path::PathBuf;

fn liberation_ttf() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/internal/standard14_outlines.fonts/LiberationSans-Regular.ttf")
}

#[test]
fn find_font_name_and_base_font() {
    let f: Font = FontRepository::find_font("Helvetica");
    assert_eq!(f.font_name(), "Helvetica");
    assert_eq!(f.base_font(), "Helvetica");
}

#[test]
fn name_resolved_fonts_are_not_embedded_or_subset() {
    let mut f = FontRepository::find_font("Times-Roman");
    assert!(!f.is_embedded());
    assert!(!f.is_subset());
    f.set_is_embedded(true);
    f.set_is_subset(true);
    assert!(f.is_embedded());
    assert!(f.is_subset());
}

#[test]
fn open_font_loads_embeddable_truetype() {
    let f = FontRepository::open_font(liberation_ttf()).expect("loads");
    assert!(f.is_embedded());
    assert!(f.font_name().contains("Liberation"));
}

#[test]
fn open_font_rejects_bad_input() {
    assert!(matches!(
        FontRepository::open_font("/no/such/font.ttf"),
        Err(Error::Io(_))
    ));
}
