// Public-API smoke driver — A-6: printer marks + AnnotationSelector
// (closes the annotations cluster).

use aspose_pdf_foss::{
    Annotation, AnnotationCollection, AnnotationSelector, AnnotationType, BleedMarkAnnotation,
    ColorBarAnnotation, ColorsOfCMYK, CornerPrinterMarkAnnotation, Document,
    PageInformationAnnotation, PrinterMarkCornerPosition, PrinterMarkSidePosition,
    RegistrationMarkAnnotation, Rectangle, TextAnnotation, TrimMarkAnnotation,
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
fn printer_mark_types() {
    let doc = page_doc();
    let page = doc.pages().get(1).unwrap();
    let r = Rectangle::new(0.0, 0.0, 20.0, 20.0, false);

    let mut cb = ColorBarAnnotation::on_page(&page, r, ColorsOfCMYK::Cyan);
    assert_eq!(cb.annotation_type(), AnnotationType::ColorBar);
    assert_eq!(cb.color_of_cmyk(), ColorsOfCMYK::Cyan);
    cb.set_color_of_cmyk(ColorsOfCMYK::Black);
    assert_eq!(cb.color_of_cmyk(), ColorsOfCMYK::Black);

    assert_eq!(
        PageInformationAnnotation::on_page(&page, r).annotation_type(),
        AnnotationType::PageInformation
    );

    let mut reg = RegistrationMarkAnnotation::on_page(&page, PrinterMarkSidePosition::Top);
    assert_eq!(reg.annotation_type(), AnnotationType::RegistrationMark);
    assert_eq!(reg.position(), PrinterMarkSidePosition::Top);
    reg.set_position(PrinterMarkSidePosition::Bottom);
    assert_eq!(reg.position(), PrinterMarkSidePosition::Bottom);
}

#[test]
fn corner_marks() {
    let doc = page_doc();
    let page = doc.pages().get(1).unwrap();
    let mut tm = TrimMarkAnnotation::on_page(&page, PrinterMarkCornerPosition::TopLeft);
    assert_eq!(tm.annotation_type(), AnnotationType::TrimMark);
    assert_eq!(tm.position(), PrinterMarkCornerPosition::TopLeft);
    tm.set_position(PrinterMarkCornerPosition::BottomRight);
    assert_eq!(tm.position(), PrinterMarkCornerPosition::BottomRight);

    let bm = BleedMarkAnnotation::on_page(&page, PrinterMarkCornerPosition::TopRight);
    assert_eq!(bm.annotation_type(), AnnotationType::BleedMark);
    assert_eq!(bm.position(), PrinterMarkCornerPosition::TopRight);
}

#[test]
fn selector_visits_directly() {
    let doc = page_doc();
    let page = doc.pages().get(1).unwrap();
    let mut sel = AnnotationSelector::new();
    let cb = ColorBarAnnotation::on_page(
        &page,
        Rectangle::new(0.0, 0.0, 5.0, 5.0, false),
        ColorsOfCMYK::Magenta,
    );
    sel.visit_color_bar_annotation(&cb);
    assert_eq!(sel.selected(), &[AnnotationType::ColorBar]);
}

#[test]
fn collection_accept_dispatches() {
    let doc = Document::new();
    let col = AnnotationCollection::new();
    col.add(Box::new(TextAnnotation::new(&doc)));
    let page_doc = page_doc();
    let page = page_doc.pages().get(1).unwrap();
    col.add(Box::new(ColorBarAnnotation::on_page(
        &page,
        Rectangle::new(0.0, 0.0, 5.0, 5.0, false),
        ColorsOfCMYK::Yellow,
    )));

    let mut sel = AnnotationSelector::new();
    col.accept(&mut sel);
    // Every annotation in the collection was visited, in order.
    assert_eq!(
        sel.selected(),
        &[AnnotationType::Text, AnnotationType::ColorBar]
    );
}

/// Five printer-mark kinds share the `/PrinterMark` subtype, so without
/// §12.5.6.21's `/MN` the file could not say which mark it was and a
/// reopened document collapsed all five into the general kind.
#[test]
fn a_printer_mark_carries_the_name_that_tells_it_apart() {
    let doc = page_doc();
    {
        let page = doc.pages().get(1).unwrap();
        page.annotations().add(Box::new(TrimMarkAnnotation::on_page(
            &page,
            PrinterMarkCornerPosition::TopLeft,
        )));
        page.annotations().add(Box::new(ColorBarAnnotation::on_page(
            &page,
            Rectangle::new(0.0, 0.0, 100.0, 10.0, false),
            ColorsOfCMYK::Cyan,
        )));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/PrinterMark"), "both are printer marks");
    assert!(text.contains("/MN /TrimMark"), "one named as a trim mark");
    assert!(text.contains("/MN /ColorBar"), "the other as a colour bar");

    // And the name is what a reopen reads the kind back from.
    let path = common::unique_temp("aspose_rust_printer_marks.pdf");
    std::fs::write(&path, &out).unwrap();
    let reopened = Document::open(&path).unwrap();
    let page = reopened.pages().get(1).unwrap();
    // The collection is indexed from zero.
    let kinds: Vec<AnnotationType> = (0..page.annotations().count())
        .filter_map(|i| page.annotations().get(i).ok())
        .map(|a| a.annotation_type())
        .collect();
    assert!(kinds.contains(&AnnotationType::TrimMark), "the trim mark survives");
    assert!(kinds.contains(&AnnotationType::ColorBar), "and so does the bar");
    let _ = std::fs::remove_file(&path);
}

#[path = "../common/mod.rs"]
mod common;
