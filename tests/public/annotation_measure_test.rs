// Public-API smoke driver — Annotation measurement dictionary.
//
// The Measure value type (ScaleRatio / Origin / XYFactor), its
// per-axis NumberFormat lists, and the §12.9 dictionary all three
// carriers write. The Annotation-taking constructor is dropped.

use aspose_pdf_foss::{
    Document, FractionStyle, LineAnnotation, Measure, NumberFormat, NumberFormatList, Point,
    PolyAnnotation, PolygonAnnotation, Rectangle,
};
use std::path::PathBuf;

fn single_page() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pages_tree/single.pdf")
}

fn miles() -> NumberFormatList {
    let mut f = NumberFormat::default();
    f.set_unit_label("mi".into());
    f.set_convresion_factor(0.5);
    f.set_precision(100);
    f.set_thousands_separator(",".into());
    f.set_before_text("~".into());
    let mut list = NumberFormatList::default();
    list.add(f);
    list
}

#[test]
fn measure_value_accessors_round_trip() {
    let mut m = Measure::default();
    assert_eq!(m.scale_ratio(), "");
    assert_eq!(m.origin(), Point::new(0.0, 0.0));
    assert_eq!(m.xy_factor(), 0.0);

    m.set_scale_ratio("1 in = 1 mi".into());
    m.set_origin(Point::new(2.0, 3.0));
    m.set_xy_factor(1.5);
    assert_eq!(m.scale_ratio(), "1 in = 1 mi");
    assert_eq!(m.origin(), Point::new(2.0, 3.0));
    assert_eq!(m.xy_factor(), 1.5);
}

#[test]
fn line_annotation_measure_round_trip() {
    let doc = Document::new();
    let mut line = LineAnnotation::new(&doc, Point::new(0.0, 0.0), Point::new(10.0, 0.0));
    assert_eq!(line.measure().scale_ratio(), ""); // default

    let mut m = Measure::default();
    m.set_scale_ratio("1:100".into());
    line.set_measure(m);
    assert_eq!(line.measure().scale_ratio(), "1:100");
}

/// `/Measure` was declared unwritable, and the declaration was the
/// thing that was wrong: §12.9 Table 261 is plain dictionary values
/// throughout, and Table 262's number formats are an array of
/// dictionaries, so none of it needs an object of its own. Until this
/// landed, a dimension line reached the file with no scale at all and
/// every one of NumberFormat's ten properties stopped at the object.
#[test]
fn a_measured_line_carries_its_measurement_dictionary() {
    let doc = Document::open(single_page()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut line =
            LineAnnotation::on_page(
                &page,
                Rectangle::new(0.0, 0.0, 100.0, 20.0, false),
                Point::new(10.0, 10.0),
                Point::new(90.0, 10.0),
            );
        let mut m = Measure::default();
        m.set_scale_ratio("1 in = 0.5 mi".into());
        m.set_distance_format(miles());
        m.set_origin(Point::new(5.0, 6.0));
        line.set_measure(m);
        page.annotations().add(Box::new(line));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);

    assert!(text.contains("/Measure <<"), "the dictionary is written");
    assert!(text.contains("/Subtype /RL"), "as the one measurement subtype");
    assert!(text.contains("/R (1 in = 0.5 mi)"), "with the scale ratio");
    assert!(text.contains("/O [ 5.0 6.0 ]"), "and the origin, which was moved");
    assert!(text.contains("/Type /NumberFormat"), "the distance format");
    assert!(text.contains("/U (mi)"), "its required unit label");
    assert!(text.contains("/C 0.5"), "its required conversion factor");
    assert!(text.contains("/F /D"), "decimal by default");
    assert!(text.contains("/D 100"), "whose /D is the precision");
    assert!(text.contains("/RT (,)"), "the thousands separator");
    assert!(text.contains("/PS (~)"), "and the text before the value");
}

/// Table 262 gives the two fractional carriers one key: `/D` is the
/// precision of a decimal format and the denominator of a fractional
/// one, so which property is written depends on `/F`.
#[test]
fn a_fractional_format_writes_its_denominator_in_the_same_key() {
    let doc = Document::open(single_page()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut poly = PolygonAnnotation::on_page(
            &page,
            Rectangle::new(0.0, 0.0, 100.0, 100.0, false),
            vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(50.0, 100.0)],
        );
        let mut f = NumberFormat::default();
        f.set_unit_label("ft".into());
        f.set_convresion_factor(1.0);
        f.set_precision(1000);
        f.set_denominator(16);
        f.set_force_denominator(true);
        f.set_fraction_displayment(FractionStyle::ShowAsFraction);
        let mut list = NumberFormatList::default();
        list.add(f);
        let mut m = Measure::default();
        m.set_area_format(list);
        poly.set_measure(m);
        page.annotations().add(Box::new(poly));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/F /F"), "a fractional format");
    assert!(text.contains("/D 16"), "writes the denominator, not the precision");
    assert!(!text.contains("/D 1000"), "the precision is not the fractional key here");
    assert!(text.contains("/FD true"), "and the forced denominator");
}

/// Table 262 makes `/U` and `/C` required, so a default-constructed
/// format is a placeholder, not a format: writing it would produce an
/// invalid dictionary, and an axis with nothing but those gets no array.
/// A measure that says nothing at all writes no `/Measure` either.
#[test]
fn an_empty_measurement_writes_nothing() {
    let doc = Document::open(single_page()).unwrap();
    {
        let page = doc.pages().get(1).unwrap();
        let mut line =
            LineAnnotation::on_page(
                &page,
                Rectangle::new(0.0, 0.0, 100.0, 20.0, false),
                Point::new(10.0, 10.0),
                Point::new(90.0, 10.0),
            );
        let mut list = NumberFormatList::default();
        list.add(NumberFormat::default());
        let mut m = Measure::default();
        m.set_x_format(list);
        // An x/y factor with no /Y array beside it is meaningless.
        m.set_xy_factor(2.0);
        line.set_measure(m);
        page.annotations().add(Box::new(line));
    }
    let mut out = Vec::new();
    doc.save_to_writer(&mut out).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/Subtype /Line"), "the annotation is written");
    assert!(!text.contains("/Measure"), "and carries no measurement dictionary");
    assert!(!text.contains("/CYX"), "nor an x/y factor without a y axis");
}

