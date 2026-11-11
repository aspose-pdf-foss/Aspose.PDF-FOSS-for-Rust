// Public-API smoke driver — figure annotations (A-2b).
//
// SquareAnnotation / CircleAnnotation (CommonFigure) and
// LineAnnotation. Ports the cpp drawing_family_smoke_test (these
// enums — CaptionPosition / LineIntent / LineEnding — match the
// canonical ordinals, so they are pinned directly).

use aspose_pdf_foss::{
    Annotation, AnnotationType, CaptionPosition, CircleAnnotation, Color, CommonFigureAnnotation,
    Document, LineAnnotation, LineEnding, LineIntent, Point, Rectangle, SquareAnnotation,
};

#[test]
fn square_and_circle_types() {
    let doc = Document::new();
    assert_eq!(
        SquareAnnotation::new(&doc).annotation_type(),
        AnnotationType::Square
    );
    assert_eq!(
        CircleAnnotation::new(&doc).annotation_type(),
        AnnotationType::Circle
    );
}

#[test]
fn common_figure_accessors() {
    let doc = Document::new();
    let mut c = CircleAnnotation::new(&doc);
    assert_eq!(c.interior_color().a(), 0.0); // empty by default
    c.set_interior_color(Color::red());
    assert_eq!(c.interior_color().a(), 1.0);

    c.set_frame(Rectangle::new(1.0, 2.0, 3.0, 20.0, false));
    assert_eq!(c.frame().llx(), 1.0);
    assert_eq!(c.frame().ury(), 20.0);
}

#[test]
fn line_construct_and_accessors() {
    let doc = Document::new();
    let mut l = LineAnnotation::new(&doc, Point::new(0.0, 0.0), Point::new(100.0, 50.0));
    assert_eq!(l.annotation_type(), AnnotationType::Line);
    assert_eq!(l.starting().x(), 0.0);
    assert_eq!(l.ending().x(), 100.0);
    assert_eq!(l.ending().y(), 50.0);
    assert_eq!(l.starting_style(), LineEnding::None);
    assert_eq!(l.ending_style(), LineEnding::None);
    assert!(!l.show_caption());
    assert_eq!(l.caption_position(), CaptionPosition::Inline);
    assert_eq!(l.intent(), LineIntent::Undefined);

    l.set_starting_style(LineEnding::Diamond);
    l.set_ending_style(LineEnding::OpenArrow);
    l.set_leader_line(5.0);
    l.set_leader_line_extension(3.0);
    l.set_leader_line_offset(1.0);
    l.set_show_caption(true);
    l.set_caption_offset(Point::new(2.0, 4.0));
    l.set_caption_position(CaptionPosition::Top);
    l.set_intent(LineIntent::LineArrow);
    assert_eq!(l.starting_style(), LineEnding::Diamond);
    assert_eq!(l.ending_style(), LineEnding::OpenArrow);
    assert_eq!(l.leader_line(), 5.0);
    assert_eq!(l.leader_line_extension(), 3.0);
    assert_eq!(l.leader_line_offset(), 1.0);
    assert!(l.show_caption());
    assert_eq!(l.caption_offset().y(), 4.0);
    assert_eq!(l.caption_position(), CaptionPosition::Top);
    assert_eq!(l.intent(), LineIntent::LineArrow);
}

fn single_page() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pages_tree/single.pdf")
}

/// A line annotation wrote only its endpoints and ending styles, so
/// the rest of §12.5.6.7 Table 177 — the ending fill, the leader
/// lines, the caption and the intent — was accepted, stored and
/// dropped on save.
#[test]
fn a_line_annotation_carries_the_rest_of_its_table() {
    let doc = Document::open(single_page()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut l = LineAnnotation::on_page(
            &page,
            Rectangle::new(0.0, 0.0, 200.0, 200.0, false),
            Point::new(10.0, 10.0),
            Point::new(100.0, 10.0),
        );
        l.set_starting_style(LineEnding::OpenArrow);
        l.set_interior_color(Color::from_rgb(0.0, 0.0, 1.0));
        l.set_leader_line(12.0);
        l.set_leader_line_extension(4.0);
        l.set_leader_line_offset(2.0);
        l.set_show_caption(true);
        l.set_caption_position(CaptionPosition::Top);
        l.set_caption_offset(Point::new(3.0, -5.0));
        l.set_intent(LineIntent::LineDimension);
        page.annotations().add(Box::new(l));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    assert!(text.contains("/IC ["), "the ending fill colour");
    assert!(text.contains("/LL 12"), "the leader-line length");
    assert!(text.contains("/LLE 4"), "its extension");
    assert!(text.contains("/LLO 2"), "its offset");
    assert!(text.contains("/Cap true"), "the caption flag");
    assert!(text.contains("/CP /Top"), "the caption position");
    assert!(text.contains("/CO ["), "the caption offset");
    assert!(text.contains("/IT /LineDimension"), "the intent");
}

/// /CP and /CO are meaningful only when /Cap is true (§12.5.6.7), so
/// a line with no caption writes none of the three. An undefined
/// intent and zero leader lines are likewise the file saying nothing.
#[test]
fn a_line_without_a_caption_writes_no_caption_entries() {
    let doc = Document::open(single_page()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut l = LineAnnotation::on_page(
            &page,
            Rectangle::new(0.0, 0.0, 200.0, 200.0, false),
            Point::new(10.0, 10.0),
            Point::new(100.0, 10.0),
        );
        l.set_caption_position(CaptionPosition::Top);
        l.set_caption_offset(Point::new(3.0, -5.0));
        page.annotations().add(Box::new(l));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    assert!(text.contains("/L ["), "the line itself is written");
    assert!(!text.contains("/Cap"), "no caption flag");
    assert!(!text.contains("/CP"), "no caption position");
    assert!(!text.contains("/CO"), "no caption offset");
    assert!(!text.contains("/IT"), "no intent");
    assert!(!text.contains("/LL"), "no leader line");
}

/// A square or circle wrote neither its interior fill nor the frame
/// between its rectangle and the figure drawn inside it (§12.5.6.8).
#[test]
fn a_figure_annotation_carries_its_fill_and_frame() {
    let doc = Document::open(single_page()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut sq = SquareAnnotation::on_page(
            &page,
            Rectangle::new(0.0, 0.0, 100.0, 100.0, false),
        );
        sq.set_interior_color(Color::from_rgb(1.0, 0.0, 0.0));
        sq.set_frame(Rectangle::new(2.0, 3.0, 4.0, 5.0, false));
        page.annotations().add(Box::new(sq));

        let mut ci = CircleAnnotation::on_page(
            &page,
            Rectangle::new(0.0, 0.0, 50.0, 50.0, false),
        );
        ci.set_interior_color(Color::from_rgb(0.0, 1.0, 0.0));
        page.annotations().add(Box::new(ci));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    assert_eq!(
        text.matches("/IC [").count(),
        2,
        "both figures carry an interior colour"
    );
    assert!(text.contains("/RD ["), "the square carries its frame");
    // The circle's frame is all zero — the figure fills its rectangle,
    // which an absent /RD already says.
    assert_eq!(text.matches("/RD [").count(), 1, "and only the square");
}
