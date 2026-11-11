// Public-API smoke driver — page labels.
//
// Ports the cpp page_labels_smoke_test (NumberingStyle ordinals,
// PageLabel defaults, range floor lookup + mutation) + the
// SaveAndReopenRoundTrip persistence test via Document.page_labels().

use aspose_pdf_foss::{Document, NumberingStyle, PageLabel, PageLabelCollection};
use std::path::{Path, PathBuf};

fn two_pages() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/two_pages.pdf")
}

#[test]
fn document_save_reopen_round_trip() {
    let out = common::unique_temp("aspose_rust_page_labels.pdf");
    {
        let mut doc = Document::open(two_pages()).unwrap();
        assert!(doc.page_labels().get_pages().is_empty());
        let labels = doc.page_labels_mut();

        let mut front = PageLabel::new();
        front.set_numbering_style(NumberingStyle::NumeralsRomanLowercase);
        labels.update_label(0, front);

        let mut body = PageLabel::new();
        body.set_numbering_style(NumberingStyle::NumeralsArabic);
        body.set_starting_value(1);
        body.set_prefix("A-".into());
        labels.update_label(1, body);

        doc.save_to(&out).unwrap();
    }
    let doc = Document::open(&out).unwrap();
    let labels = doc.page_labels();
    assert_eq!(labels.get_pages().len(), 2);
    assert_eq!(labels.get_label(0).numbering_style(), NumberingStyle::NumeralsRomanLowercase);
    let b = labels.get_label(1);
    assert_eq!(b.numbering_style(), NumberingStyle::NumeralsArabic);
    assert_eq!(b.prefix(), "A-");
    let _ = std::fs::remove_file(&out);
}

#[test]
fn numbering_style_ordinals() {
    assert_eq!(NumberingStyle::NumeralsArabic as i32, 0);
    assert_eq!(NumberingStyle::NumeralsRomanUppercase as i32, 1);
    assert_eq!(NumberingStyle::NumeralsRomanLowercase as i32, 2);
    assert_eq!(NumberingStyle::LettersUppercase as i32, 3);
    assert_eq!(NumberingStyle::LettersLowercase as i32, 4);
    assert_eq!(NumberingStyle::None as i32, 5);
}

#[test]
fn page_label_defaults() {
    let d = PageLabel::new();
    assert_eq!(d.starting_value(), 1);
    assert_eq!(d.numbering_style(), NumberingStyle::NumeralsArabic);
    assert!(d.prefix().is_empty());
}

#[test]
fn range_lookup_and_mutation() {
    let mut labels = PageLabelCollection::new();
    assert!(labels.get_pages().is_empty());

    let mut roman = PageLabel::new();
    roman.set_numbering_style(NumberingStyle::NumeralsRomanLowercase);
    roman.set_prefix("pre-".into());
    labels.update_label(0, roman);

    let mut arabic = PageLabel::new();
    arabic.set_starting_value(1);
    labels.update_label(2, arabic);

    assert_eq!(labels.get_pages(), vec![0, 2]);

    assert_eq!(
        labels.get_label(0).numbering_style(),
        NumberingStyle::NumeralsRomanLowercase
    );
    // index 1 is still in the [0, 2) range — floor lookup.
    assert_eq!(labels.get_label(1).prefix(), "pre-");
    assert_eq!(
        labels.get_label(2).numbering_style(),
        NumberingStyle::NumeralsArabic
    );

    assert!(labels.remove_label(2));
    assert!(!labels.remove_label(2));
    assert_eq!(labels.get_pages().len(), 1);
}

#[test]
fn get_label_below_first_range_is_default() {
    let mut labels = PageLabelCollection::new();
    let mut body = PageLabel::new();
    body.set_numbering_style(NumberingStyle::LettersUppercase);
    labels.update_label(5, body);
    // No range covers index 0 — a default label is returned.
    let d = labels.get_label(0);
    assert_eq!(d.starting_value(), 1);
    assert_eq!(d.numbering_style(), NumberingStyle::NumeralsArabic);
}

#[path = "../common/mod.rs"]
mod common;
