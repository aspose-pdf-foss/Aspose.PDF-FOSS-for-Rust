// Public-API smoke driver — TextSearchOptions + its use by
// TextFragmentAbsorber's phrase-with-options constructor.

use aspose_pdf_foss::{Document, Rectangle, TextFragmentAbsorber, TextSearchOptions};
use std::path::PathBuf;

fn hello_world_pdf() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/text_extractor/hello_world.pdf")
}

#[test]
fn options_round_trip() {
    let mut o = TextSearchOptions::new(true);
    assert!(o.is_regular_expression_used());
    assert!(o.rectangle().is_none());

    o.set_limit_to_page_bounds(true);
    o.set_use_font_engine_encoding(true);
    o.set_ignore_shadow_text(true);
    o.set_log_text_extraction_errors(true);
    o.set_ignore_resource_font_errors(true);
    o.set_search_for_text_related_graphics(true);
    o.set_search_in_annotations(true);
    o.set_stored_graphic_elements_max_count(42);
    o.set_rectangle(Rectangle::new(1.0, 2.0, 3.0, 4.0, false));

    assert!(o.limit_to_page_bounds());
    assert!(o.use_font_engine_encoding());
    assert!(o.ignore_shadow_text());
    assert!(o.log_text_extraction_errors());
    assert!(o.ignore_resource_font_errors());
    assert!(o.search_for_text_related_graphics());
    assert!(o.search_in_annotations());
    assert_eq!(o.stored_graphic_elements_max_count(), 42);
    assert_eq!(o.rectangle().unwrap().urx(), 3.0);
}

#[test]
fn constructors() {
    assert!(!TextSearchOptions::new(false).is_regular_expression_used());

    let r = Rectangle::new(0.0, 0.0, 10.0, 10.0, false);
    let a = TextSearchOptions::with_rectangle(r);
    assert!(a.rectangle().is_some());
    assert!(!a.is_regular_expression_used());

    let b = TextSearchOptions::with_rectangle_and_regex(r, true);
    assert!(b.rectangle().is_some());
    assert!(b.is_regular_expression_used());
}

#[test]
fn rectangle_scope_filters_matches() {
    let doc = Document::open(hello_world_pdf()).unwrap();

    // Locate "World" unscoped to learn its rectangle.
    let mut plain = TextFragmentAbsorber::with_phrase("World");
    plain.visit_document(&doc).unwrap();
    let world = plain.text_fragments().get(1).unwrap().rectangle();

    // A scope overlapping World keeps the match.
    let inside = TextSearchOptions::with_rectangle(world);
    let mut a = TextFragmentAbsorber::with_phrase_and_options("World", inside);
    a.visit_document(&doc).unwrap();
    assert_eq!(a.text_fragments().count(), 1);

    // A scope far from any text drops the match.
    let elsewhere = Rectangle::new(-100.0, -100.0, -90.0, -90.0, false);
    let outside = TextSearchOptions::with_rectangle(elsewhere);
    let mut b = TextFragmentAbsorber::with_phrase_and_options("World", outside);
    b.visit_document(&doc).unwrap();
    assert_eq!(b.text_fragments().count(), 0);
}

/// `limit_to_page_bounds` drops matches outside the visible page. Text
/// can sit beyond the crop box — trimmed marks, content left over from
/// a resize — and is extracted all the same, so without the limit a
/// search answers with runs a reader never sees. Proved by shrinking
/// the crop box until the text falls outside it.
#[test]
fn the_page_bounds_limit_drops_text_outside_the_crop_box() {
    let found_with_limit = |crop: Rectangle, limit: bool| {
        let doc = Document::open(hello_world_pdf()).unwrap();
        doc.pages().get(1).unwrap().set_crop_box(crop);
        let mut options = TextSearchOptions::new(false);
        options.set_limit_to_page_bounds(limit);
        let mut a = TextFragmentAbsorber::with_phrase_and_options("World", options);
        a.visit_document(&doc).unwrap();
        a.text_fragments().count()
    };

    // A crop box covering the whole page keeps the match either way.
    let whole = Rectangle::new(0.0, 0.0, 612.0, 792.0, false);
    assert_eq!(found_with_limit(whole, true), 1, "inside the bounds");
    assert_eq!(found_with_limit(whole, false), 1, "and unrestricted");

    // A crop box in the bottom-left corner leaves the text outside it.
    let corner = Rectangle::new(0.0, 0.0, 20.0, 20.0, false);
    assert_eq!(found_with_limit(corner, true), 0, "outside the bounds, dropped");
    assert_eq!(
        found_with_limit(corner, false), 1,
        "and still found when the limit is off — which is the old behaviour"
    );
}

/// A pattern search answered with a literal one is a wrong answer, not
/// a missing feature: `\d+` would find nothing and report success. No
/// peer FOSS track ships a search-options surface at all, so this stays
/// outside the parity ceiling rather than becoming a regex engine of
/// our own — and the flag refuses loudly instead.
#[test]
#[should_panic(expected = "text_search_regular_expressions")]
fn a_regular_expression_search_refuses_rather_than_answering_literally() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut a = TextFragmentAbsorber::with_phrase_and_options(
        "W.rld",
        TextSearchOptions::new(true),
    );
    let _ = a.visit_document(&doc);
}

/// Without the flag the same pattern is what it looks like: a literal
/// phrase, which this text does not contain.
#[test]
fn a_literal_search_for_a_pattern_finds_nothing() {
    let doc = Document::open(hello_world_pdf()).unwrap();
    let mut a = TextFragmentAbsorber::with_phrase_and_options(
        "W.rld",
        TextSearchOptions::new(false),
    );
    a.visit_document(&doc).unwrap();
    assert_eq!(a.text_fragments().count(), 0);
}
