// Public-API smoke driver — first concrete annotations (A-2).
//
// TextAnnotation (markup) and LinkAnnotation (direct). Ports the
// markup + concrete behaviour from the cpp markup / concrete smoke
// tests, using the canonical 26.4.0 enum ordinals (the cpp tests
// assert older, drifted enum values — not matched here).

use aspose_pdf_foss::{
    Annotation, AnnotationCollection, AnnotationState, AnnotationStateModel, AnnotationType,
    Document, GoToURIAction, HighlightingMode, LinkAnnotation, MarkupAnnotation, Rectangle,
    ReplyType, TextAnnotation, TextIcon, XYZExplicitDestination,
};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn text_annotation_construct_and_accessors() {
    let doc = Document::new();
    let mut a = TextAnnotation::new(&doc);
    assert_eq!(a.annotation_type(), AnnotationType::Text);
    assert!(!a.open());
    assert_eq!(a.icon(), TextIcon::Note);
    a.set_open(true);
    a.set_icon(TextIcon::Comment);
    assert!(a.open());
    assert_eq!(a.icon(), TextIcon::Comment);
}

#[test]
fn markup_defaults_and_round_trip() {
    let doc = Document::new();
    let mut m = TextAnnotation::new(&doc);
    assert_eq!(m.title(), "");
    assert_eq!(m.rich_text(), "");
    assert_eq!(m.subject(), "");
    assert_eq!(m.opacity(), 1.0);
    assert_eq!(m.reply_type(), ReplyType::Reply);
    assert_eq!(m.get_state(), AnnotationState::None);
    assert_eq!(m.get_state_model(), AnnotationStateModel::Undefined);

    m.set_title("Alice".into());
    m.set_rich_text("<b>hi</b>".into());
    m.set_subject("Notes".into());
    m.set_opacity(0.5);
    m.set_reply_type(ReplyType::Group);
    assert_eq!(m.title(), "Alice");
    assert_eq!(m.rich_text(), "<b>hi</b>");
    assert_eq!(m.subject(), "Notes");
    assert_eq!(m.opacity(), 0.5);
    assert_eq!(m.reply_type(), ReplyType::Group);
}

#[test]
fn markup_review_and_marked_state() {
    let doc = Document::new();
    let mut m = TextAnnotation::new(&doc);
    m.set_review_state(AnnotationState::Accepted, "Bob".into());
    assert_eq!(m.get_state(), AnnotationState::Accepted);
    assert_eq!(m.get_state_model(), AnnotationStateModel::Review);

    m.set_marked_state(true);
    assert_eq!(m.get_state(), AnnotationState::Marked);
    assert_eq!(m.get_state_model(), AnnotationStateModel::Marked);

    m.clear_state();
    assert_eq!(m.get_state(), AnnotationState::None);
    assert_eq!(m.get_state_model(), AnnotationStateModel::Undefined);
}

#[test]
fn link_annotation_construct_and_accessors() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    let page = doc.pages().get(1).unwrap();
    let mut link = LinkAnnotation::on_page(&page, Rectangle::new(0.0, 0.0, 100.0, 20.0, false));
    assert_eq!(link.annotation_type(), AnnotationType::Link);
    assert!(link.action().is_none());
    assert!(link.destination().is_none());
    assert_eq!(link.highlighting(), HighlightingMode::None);

    link.set_action(Box::new(GoToURIAction::new("https://example.org".into())));
    link.set_destination(Box::new(XYZExplicitDestination::new(1, 0.0, 700.0, 1.0)));
    link.set_highlighting(HighlightingMode::Invert);
    assert_eq!(link.action().unwrap().to_string(), "https://example.org");
    assert_eq!(link.destination().unwrap().to_string(), "1 XYZ 0 700 1");
    assert_eq!(link.highlighting(), HighlightingMode::Invert);
}

#[test]
fn annotations_in_collection_are_polymorphic() {
    let doc = Document::new();
    let col = AnnotationCollection::new();
    let mut t = TextAnnotation::new(&doc);
    t.set_name("note".into());
    col.add(Box::new(t));

    let doc2 = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    let page = doc2.pages().get(1).unwrap();
    let mut l = LinkAnnotation::on_page(&page, Rectangle::new(0.0, 0.0, 10.0, 10.0, false));
    l.set_name("lnk".into());
    col.add(Box::new(l));

    assert_eq!(col.count(), 2);
    assert_eq!(col.get(0).unwrap().annotation_type(), AnnotationType::Text);
    assert_eq!(col.get(1).unwrap().annotation_type(), AnnotationType::Link);
    assert!(col.find_by_name("lnk").is_some());
}

/// `/A` was written by nothing, so a link with an action reached the
/// file as a rectangle that does nothing and a bookmark with an action
/// lost it entirely. §12.6: an action dictionary is `/S` plus the one
/// entry that carries its payload.
#[test]
fn a_link_carries_its_action() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut link = LinkAnnotation::on_page(&page, Rectangle::new(10.0, 10.0, 90.0, 30.0, false));
        link.set_action(Box::new(GoToURIAction::new("https://example.invalid/a".to_string())));
        page.annotations().add(Box::new(link));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/A <<"), "the action is a dictionary");
    assert!(text.contains("/S /URI"), "naming its subtype");
    assert!(text.contains("https://example.invalid/a"), "and carrying the target");
}

/// An action with nothing in it is not written at all: a dictionary
/// with only `/S` tells a viewer nothing it can act on.
#[test]
fn an_empty_action_is_not_written() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut link = LinkAnnotation::on_page(&page, Rectangle::new(10.0, 10.0, 90.0, 30.0, false));
        link.set_action(Box::new(GoToURIAction::new(String::new())));
        page.annotations().add(Box::new(link));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    assert!(
        !String::from_utf8_lossy(&out).contains("/A <<"),
        "an action with no target should leave no /A"
    );
}

/// A note's icon is the only visible form it has, and its review state
/// is what a comment workflow reads. Both were stored and never
/// written, so every note reached the file as a default closed /Note
/// with no state (§12.5.6.4 Table 172).
#[test]
fn a_text_annotation_carries_its_icon_open_state_and_review_state() {
    use aspose_pdf_foss::TextAnnotation;

    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut note = TextAnnotation::on_page(&page, Rectangle::new(5.0, 5.0, 25.0, 25.0, false));
        note.set_icon(TextIcon::Help);
        note.set_open(true);
        note.set_review_state_value(AnnotationState::Accepted);
        page.annotations().add(Box::new(note));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    assert!(text.contains("/Name /Help"), "the icon a viewer draws");
    assert!(text.contains("/Open true"), "the window starts open");
    // §12.5.6.4: /StateModel is required whenever /State is present.
    assert!(text.contains("/StateModel /Review"), "the state model");
    assert!(text.contains("/State /Accepted"), "and the state itself");
}

/// Clearing the state removes both entries, since /State may not stand
/// without /StateModel.
#[test]
fn a_cleared_state_writes_neither_entry() {
    use aspose_pdf_foss::TextAnnotation;

    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut note = TextAnnotation::on_page(&page, Rectangle::new(5.0, 5.0, 25.0, 25.0, false));
        note.set_marked_state(true);
        note.clear_state();
        page.annotations().add(Box::new(note));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/Text"), "the note itself is written");
    assert!(!text.contains("/State"), "with no state and no model");
}

/// Both date entries were held back by a reason that stopped being true
/// on the day `DateTime` learnt to format a PDF date for a signature:
/// the notes still said the type had no formatter and no unset state.
/// §12.5.2 Table 164 gives every annotation `/M`, and §12.5.6.2
/// Table 170 gives a markup annotation `/CreationDate`.
#[test]
fn an_annotation_carries_the_dates_it_was_given() {
    use aspose_pdf_foss::DateTime;
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut t = TextAnnotation::on_page(&page, Rectangle::new(10.0, 10.0, 30.0, 30.0, false));
        t.set_modified(DateTime::new(2026, 11, 4, 14, 30, 0));
        t.set_creation_date(DateTime::new(2026, 10, 1, 9, 15, 30));
        page.annotations().add(Box::new(t));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/M (D:20261104143000Z)"), "the modification date");
    assert!(
        text.contains("/CreationDate (D:20261001091530Z)"),
        "and the creation date, which is a different entry"
    );
}

/// The other half of the same call: an annotation the caller gave no
/// date must carry neither entry rather than a date nobody stated —
/// the defect the hard-coded signature date was.
#[test]
fn an_annotation_given_no_date_carries_none() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let t = TextAnnotation::on_page(&page, Rectangle::new(10.0, 10.0, 30.0, 30.0, false));
        page.annotations().add(Box::new(t));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/Subtype /Text"), "the annotation is written");
    assert!(!text.contains("/M (D:"), "and carries no modification date");
    assert!(!text.contains("/CreationDate"), "nor a creation date");
}
