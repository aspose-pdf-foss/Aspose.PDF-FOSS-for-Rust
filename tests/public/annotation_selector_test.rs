// Public-API smoke driver — `AnnotationSelector` (ports the cpp
// annotation_selector_smoke_test). Covers default + one-arg
// construction, the `accept` double-dispatch wire-through on a
// concrete annotation, iteration via `AnnotationCollection::accept`,
// and that an empty collection's accept is a no-op.

use aspose_pdf_foss::{
    Annotation, AnnotationSelector, AnnotationType, CircleAnnotation, Document, SquareAnnotation,
    TextAnnotation,
};
use std::path::{Path, PathBuf};

fn hello_world() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text_extractor/hello_world.pdf")
}

#[test]
fn default_constructible() {
    let s = AnnotationSelector::new();
    assert!(s.selected().is_empty());
}

#[test]
fn one_arg_ctor_accepts_annotation() {
    let doc = Document::new();
    let a = TextAnnotation::new(&doc);
    let s = AnnotationSelector::for_annotation(&a);
    // Seeding does not record a visit; selection happens on accept.
    assert!(s.selected().is_empty());
}

#[test]
fn annotation_accept_dispatches() {
    let doc = Document::new();
    let a = TextAnnotation::new(&doc);
    let mut s = AnnotationSelector::new();
    a.accept(&mut s);
    assert_eq!(s.selected(), &[AnnotationType::Text]);
    a.accept(&mut s);
    a.accept(&mut s);
    assert_eq!(
        s.selected(),
        &[AnnotationType::Text, AnnotationType::Text, AnnotationType::Text]
    );
}

#[test]
fn annotation_collection_accept_iterates() {
    let doc = Document::open(hello_world()).unwrap();
    let __pg0 = doc.pages().get(1).unwrap();
    let annots = __pg0.annotations();
    annots.add(Box::new(TextAnnotation::new(&doc)));
    annots.add(Box::new(SquareAnnotation::new(&doc)));
    annots.add(Box::new(CircleAnnotation::new(&doc)));

    let mut s = AnnotationSelector::new();
    annots.accept(&mut s);
    assert_eq!(
        s.selected(),
        &[AnnotationType::Text, AnnotationType::Square, AnnotationType::Circle]
    );
}

#[test]
fn empty_collection_accept_is_noop() {
    let doc = Document::open(hello_world()).unwrap();
    let __pg1 = doc.pages().get(1).unwrap();
    let annots = __pg1.annotations();
    let mut s = AnnotationSelector::new();
    annots.accept(&mut s); // must not panic
    assert!(s.selected().is_empty());
}

#[test]
fn dispatch_routes_to_concrete_visit_overload() {
    // Each concrete annotation routes its `accept` to the matching
    // `visit_*` overload, recording its own type — the base-class
    // dispatch is never a generic no-op for a shipped type.
    let doc = Document::new();
    let sq = SquareAnnotation::new(&doc);
    let mut s = AnnotationSelector::new();
    sq.accept(&mut s);
    assert_eq!(s.selected(), &[AnnotationType::Square]);
}
