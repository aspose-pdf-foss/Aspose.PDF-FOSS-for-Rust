// Public-API smoke driver — A-3 annotation families.
//
// FreeText (markup) + Polygon/Polyline (poly) + Highlight/Underline/
// StrikeOut/Squiggly (text-markup). Behaviour from the cpp concrete
// / path-family smoke tests, using canonical enum ordinals (the cpp
// tests assert drifted Justification names — Left/Center here).

use aspose_pdf_foss::{
    Annotation, AnnotationType, DefaultAppearance, Document, FreeTextAnnotation, FreeTextIntent,
    HighlightAnnotation, Justification, LineEnding, MarkupAnnotation, Point, PolyAnnotation,
    PolyIntent, PolygonAnnotation, PolylineAnnotation, Rectangle, SquigglyAnnotation,
    StrikeOutAnnotation, TextMarkupAnnotation, UnderlineAnnotation,
};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn helv() -> DefaultAppearance {
    let mut da = DefaultAppearance::new();
    da.set_font_name("Helvetica".into());
    da.set_font_size(12.0);
    da
}

#[test]
fn free_text_construct_and_accessors() {
    let doc = Document::new();
    let mut a = FreeTextAnnotation::new(&doc, helv());
    assert_eq!(a.annotation_type(), AnnotationType::FreeText);
    assert_eq!(a.intent(), FreeTextIntent::Undefined);
    assert_eq!(a.starting_style(), LineEnding::None);
    assert_eq!(a.ending_style(), LineEnding::None);
    assert_eq!(a.justification(), Justification::Left);
    assert!(a.callout().is_empty());
    assert_eq!(a.default_style(), "font: Helvetica");
    assert_eq!(a.default_appearance_object().font_name(), "Helvetica");

    a.set_starting_style(LineEnding::OpenArrow);
    a.set_ending_style(LineEnding::ClosedArrow);
    a.set_justification(Justification::Center);
    a.set_intent(FreeTextIntent::FreeTextCallout);
    assert_eq!(a.starting_style(), LineEnding::OpenArrow);
    assert_eq!(a.ending_style(), LineEnding::ClosedArrow);
    assert_eq!(a.justification(), Justification::Center);
    assert_eq!(a.intent(), FreeTextIntent::FreeTextCallout);
    // markup base still works through the chain
    a.set_title("author".into());
    assert_eq!(a.title(), "author");
}

#[test]
fn polygon_and_polyline() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    let page = doc.pages().get(1).unwrap();
    let verts = vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(5.0, 8.0)];

    let mut pg = PolygonAnnotation::new(&Document::new(), verts.clone());
    assert_eq!(pg.annotation_type(), AnnotationType::Polygon);
    assert_eq!(pg.vertices().len(), 3);
    assert_eq!(pg.intent(), PolyIntent::Undefined);
    pg.set_intent(PolyIntent::PolygonCloud);
    assert_eq!(pg.intent(), PolyIntent::PolygonCloud);

    let pl = PolylineAnnotation::on_page(
        &page,
        Rectangle::new(0.0, 0.0, 10.0, 8.0, false),
        verts,
    );
    assert_eq!(pl.annotation_type(), AnnotationType::PolyLine);
    assert_eq!(pl.vertices()[1].x(), 10.0);
}

#[test]
fn text_markup_family() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    let page = doc.pages().get(1).unwrap();
    let r = Rectangle::new(0.0, 0.0, 50.0, 10.0, false);

    assert_eq!(
        HighlightAnnotation::on_page(&page, r).annotation_type(),
        AnnotationType::Highlight
    );
    assert_eq!(
        UnderlineAnnotation::on_page(&page, r).annotation_type(),
        AnnotationType::Underline
    );
    assert_eq!(
        StrikeOutAnnotation::on_page(&page, r).annotation_type(),
        AnnotationType::StrikeOut
    );

    let mut sq = SquigglyAnnotation::on_page(&page, r);
    assert_eq!(sq.annotation_type(), AnnotationType::Squiggly);
    assert!(sq.quad_points().is_empty());
    sq.set_quad_points(vec![Point::new(0.0, 0.0), Point::new(50.0, 0.0)]);
    assert_eq!(sq.quad_points().len(), 2);
    assert_eq!(sq.get_marked_text(), "");
    assert_eq!(sq.get_marked_text_fragments().count(), 0);
}

/// A text-markup annotation says which runs of text it covers through
/// `/QuadPoints` (§12.5.6.10); the rectangle alone does not. The
/// quadrilaterals were accepted and stored and never written, so every
/// highlight, underline, strike-out and squiggly reached the file
/// marking nothing.
#[test]
fn the_four_text_markup_types_carry_their_quadrilaterals() {
    let quad = vec![
        Point::new(10.0, 20.0),
        Point::new(60.0, 20.0),
        Point::new(10.0, 32.0),
        Point::new(60.0, 32.0),
    ];
    let r = Rectangle::new(10.0, 20.0, 60.0, 32.0, false);

    // One document per subtype, so each is asserted on its own bytes.
    for subtype in ["Highlight", "Underline", "StrikeOut", "Squiggly"] {
        let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
        {
            let page = doc.pages().get(1).unwrap();
            let mut a: Box<dyn TextMarkupAnnotation> = match subtype {
                "Highlight" => Box::new(HighlightAnnotation::on_page(&page, r)),
                "Underline" => Box::new(UnderlineAnnotation::on_page(&page, r)),
                "StrikeOut" => Box::new(StrikeOutAnnotation::on_page(&page, r)),
                _ => Box::new(SquigglyAnnotation::on_page(&page, r)),
            };
            a.set_quad_points(quad.clone());
            page.annotations().add(a);
        }
        let mut out = Vec::new();
        doc.save_to_writer(&mut out).unwrap();
        let text = String::from_utf8_lossy(&out);
        assert!(
            text.contains(&format!("/{subtype}")),
            "{subtype} must reach the file"
        );
        assert!(
            text.contains("/QuadPoints"),
            "{subtype} must carry its quadrilaterals"
        );
        // Eight numbers — the four points flattened, in order.
        let at = text.find("/QuadPoints").unwrap();
        let written = &text[at..at + 80];
        for n in ["10", "20", "60", "32"] {
            assert!(written.contains(n), "{subtype} /QuadPoints must hold {n}");
        }
    }
}

/// An empty quadrilateral set writes no entry: a viewer then draws
/// nothing, which is what a caller who gave no quadrilaterals asked
/// for.
#[test]
fn no_quadrilaterals_writes_no_entry() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let h = HighlightAnnotation::on_page(
            &page,
            Rectangle::new(1.0, 2.0, 3.0, 4.0, false),
        );
        page.annotations().add(Box::new(h));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/Highlight"), "the annotation itself is written");
    assert!(!text.contains("/QuadPoints"), "with no empty quadrilateral array");
}

/// A polygon's intent (§12.5.6.9 — a cloud, a dimension line) decides
/// how a viewer draws it; it was stored and never written.
#[test]
fn a_polygon_carries_its_intent() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut p = PolygonAnnotation::new(
            &doc,
            vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(5.0, 9.0)],
        );
        p.set_rect(Rectangle::new(0.0, 0.0, 10.0, 9.0, false));
        p.set_intent(PolyIntent::PolygonCloud);
        page.annotations().add(Box::new(p));

        // A polyline with no intent writes none.
        let l = PolylineAnnotation::on_page(
            &page,
            Rectangle::new(0.0, 0.0, 10.0, 10.0, false),
            vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)],
        );
        page.annotations().add(Box::new(l));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/IT /PolygonCloud"), "the polygon's intent");
    assert_eq!(text.matches("/IT ").count(), 1, "and only the polygon's");
}

/// §12.5.6.6 marks /DA required for a free-text annotation: it is the
/// operator string a viewer sets the text with, so without it the
/// contents have nothing to be drawn in. The annotation contributed no
/// entry of its own at all, losing /DA together with the quadding, the
/// style, the intent, the callout line and the text inset.
#[test]
fn a_free_text_annotation_carries_its_table() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut a = FreeTextAnnotation::on_page(
            &page,
            Rectangle::new(10.0, 10.0, 200.0, 60.0, false),
            helv(),
        );
        a.set_contents("typed here".into());
        a.set_justification(Justification::Center);
        a.set_intent(FreeTextIntent::FreeTextCallout);
        a.set_callout(vec![
            Point::new(5.0, 5.0),
            Point::new(20.0, 20.0),
            Point::new(40.0, 40.0),
        ]);
        a.set_starting_style(LineEnding::OpenArrow);
        a.set_text_rectangle(Rectangle::new(2.0, 2.0, 2.0, 2.0, false));
        page.annotations().add(Box::new(a));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    assert!(text.contains("/DA ("), "the required default appearance");
    assert!(text.contains("/Helv") || text.contains("Helvetica"), "naming its font");
    assert!(text.contains("/Q 1"), "centred quadding");
    assert!(text.contains("/DS (font: Helvetica)"), "the default style");
    assert!(text.contains("/IT /FreeTextCallout"), "the intent");
    assert!(text.contains("/CL ["), "the callout line");
    // §12.5.6.6: /LE is a single name here, not the two-name array a
    // line annotation carries.
    assert!(text.contains("/LE /OpenArrow"), "the callout's line ending");
    assert!(text.contains("/RD ["), "the text inset");
}

/// Without a callout there is no line to end, so /CL and /LE stay out
/// even when an ending style was set.
#[test]
fn a_free_text_without_a_callout_writes_no_line_ending() {
    let doc = Document::open(fixture("pages_tree/single.pdf")).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut a = FreeTextAnnotation::on_page(
            &page,
            Rectangle::new(10.0, 10.0, 200.0, 60.0, false),
            helv(),
        );
        a.set_starting_style(LineEnding::OpenArrow);
        page.annotations().add(Box::new(a));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/DA ("), "the required entry is still written");
    assert!(!text.contains("/CL"), "no callout line");
    assert!(!text.contains("/LE"), "and so no line ending");
    assert!(!text.contains("/Q "), "left quadding is the default");
}
