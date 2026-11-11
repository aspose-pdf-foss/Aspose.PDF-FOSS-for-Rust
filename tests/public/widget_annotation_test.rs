// Public-API smoke driver — WidgetAnnotation (forms F-1).
//
// The form-field widget annotation (deferred from cluster A; the
// base of the forms Field hierarchy).

use aspose_pdf_foss::{
    Annotation, AnnotationCollection, AnnotationSelector, AnnotationType, Document,
    GoToURIAction, HighlightingMode, WidgetAnnotation,
};

#[test]
fn widget_construct_and_accessors() {
    let doc = Document::new();
    let mut w = WidgetAnnotation::new(&doc);
    assert_eq!(w.annotation_type(), AnnotationType::Widget);
    assert!(!w.read_only());
    assert!(!w.required());
    assert!(w.exportable()); // default exportable
    assert_eq!(w.highlighting(), HighlightingMode::None);
    assert_eq!(w.get_checked_state_name(), "");
    assert!(w.on_activated().is_none());

    w.set_read_only(true);
    w.set_required(true);
    w.set_exportable(false);
    w.set_highlighting(HighlightingMode::Push);
    w.set_on_activated(Box::new(GoToURIAction::new("https://x".into())));
    assert!(w.read_only());
    assert!(w.required());
    assert!(!w.exportable());
    assert_eq!(w.highlighting(), HighlightingMode::Push);
    assert_eq!(w.on_activated().unwrap().to_string(), "https://x");
}

#[test]
fn widget_visits_through_selector() {
    let doc = Document::new();
    let col = AnnotationCollection::new();
    col.add(Box::new(WidgetAnnotation::new(&doc)));
    let mut sel = AnnotationSelector::new();
    col.accept(&mut sel);
    assert_eq!(sel.selected(), &[AnnotationType::Widget]);

    // Direct typed visit also records.
    let mut sel2 = AnnotationSelector::new();
    sel2.visit_widget_annotation(&WidgetAnnotation::new(&doc));
    assert_eq!(sel2.selected(), &[AnnotationType::Widget]);
}
