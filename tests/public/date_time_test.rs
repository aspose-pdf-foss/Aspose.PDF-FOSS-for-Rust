// Public-API smoke driver — DateTime carrier + its date members.
//
// The std-only DateTime value type and the un-dropped date members:
// Annotation.Modified (base trait), MarkupAnnotation.CreationDate,
// DateField.Value, Signature.Date.

use aspose_pdf_foss::{
    Annotation, DateField, DateTime, Document, LineAnnotation, MarkupAnnotation, Point, Signature,
};

#[test]
fn date_time_components() {
    let dt = DateTime::new(2026, 7, 15, 9, 30, 45);
    assert_eq!((dt.year(), dt.month(), dt.day()), (2026, 7, 15));
    assert_eq!((dt.hour(), dt.minute(), dt.second()), (9, 30, 45));
    assert_eq!(DateTime::default(), DateTime::new(0, 0, 0, 0, 0, 0));
}

#[test]
fn annotation_modified_and_creation_date() {
    let doc = Document::new();
    let mut line = LineAnnotation::new(&doc, Point::new(0.0, 0.0), Point::new(1.0, 1.0));
    assert_eq!(line.modified(), DateTime::default());
    line.set_modified(DateTime::new(2026, 1, 2, 3, 4, 5));
    assert_eq!(line.modified().year(), 2026);
    // CreationDate is on the MarkupAnnotation trait.
    line.set_creation_date(DateTime::new(2020, 6, 1, 0, 0, 0));
    assert_eq!(line.creation_date().year(), 2020);
}

#[test]
fn date_field_value() {
    let doc = Document::new();
    let mut f = DateField::new(&doc);
    assert_eq!(f.value(), DateTime::default());
    f.set_value(DateTime::new(2024, 12, 31, 23, 59, 59));
    assert_eq!(f.value().day(), 31);
}

#[test]
fn signature_date() {
    let mut s = Signature::new();
    assert_eq!(s.date(), DateTime::default());
    s.set_date(DateTime::new(2025, 3, 3, 12, 0, 0));
    assert_eq!(s.date().month(), 3);
}
