// Public-API smoke driver — BaseParagraph / MarginInfo + the
// VerticalAlignment / HorizontalAlignment enums.
//
// Ports base_paragraph_smoke_test.cpp. BaseParagraph is a trait
// (Rust has no inheritance), so its accessor surface is exercised
// through a concrete implementor — Table, which embeds the shared
// paragraph geometry. The Hyperlink scenarios are omitted (see the
// port report): Aspose.Pdf.Hyperlink is a members-less marker trait
// here with no concrete type and no default constructor.

use aspose_pdf_foss::{BaseParagraph, HorizontalAlignment, MarginInfo, Table, VerticalAlignment};

const EPS: f64 = 1e-9;
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < EPS, "{a} !~= {b}");
}

// ---- MarginInfo --------------------------------------------------

#[test]
fn margin_info_default_constructed_is_zero() {
    let m = MarginInfo::new();
    near(m.left(), 0.0);
    near(m.right(), 0.0);
    near(m.top(), 0.0);
    near(m.bottom(), 0.0);
}

#[test]
fn margin_info_param_ctor_sets_all_four() {
    // Canonical ctor parameter order is (left, bottom, right, top).
    let m = MarginInfo::with_margins(1.0, 2.0, 3.0, 4.0);
    near(m.left(), 1.0);
    near(m.bottom(), 2.0);
    near(m.right(), 3.0);
    near(m.top(), 4.0);
}

#[test]
fn margin_info_accessor_roundtrip() {
    let mut m = MarginInfo::new();
    m.set_left(10.0);
    m.set_right(20.0);
    m.set_top(30.0);
    m.set_bottom(40.0);
    near(m.left(), 10.0);
    near(m.right(), 20.0);
    near(m.top(), 30.0);
    near(m.bottom(), 40.0);
}

// ---- the alignment enums -----------------------------------------

#[test]
fn vertical_alignment_canonical_values() {
    assert_eq!(VerticalAlignment::None as i32, 0);
    assert_eq!(VerticalAlignment::Top as i32, 1);
    assert_eq!(VerticalAlignment::Center as i32, 2);
    assert_eq!(VerticalAlignment::Bottom as i32, 3);
}

#[test]
fn horizontal_alignment_canonical_values() {
    assert_eq!(HorizontalAlignment::None as i32, 0);
    assert_eq!(HorizontalAlignment::Left as i32, 1);
    assert_eq!(HorizontalAlignment::Center as i32, 2);
    assert_eq!(HorizontalAlignment::Right as i32, 3);
    assert_eq!(HorizontalAlignment::Justify as i32, 4);
    assert_eq!(HorizontalAlignment::FullJustify as i32, 5);
}

// ---- BaseParagraph (via the Table implementor) -------------------

#[test]
fn base_paragraph_default_accessors() {
    let p = Table::new();
    assert_eq!(p.vertical_alignment(), VerticalAlignment::None);
    assert_eq!(p.horizontal_alignment(), HorizontalAlignment::None);
    assert!(!p.is_first_paragraph_in_column());
    assert!(!p.is_kept_with_next());
    assert!(!p.is_in_new_page());
    assert!(!p.is_in_line_paragraph());
    assert_eq!(p.z_index(), 0);
    assert!(p.hyperlink().is_none());
    near(p.margin().left(), 0.0);
}

#[test]
fn base_paragraph_accessor_roundtrip() {
    let mut p = Table::new();
    p.set_vertical_alignment(VerticalAlignment::Center);
    p.set_horizontal_alignment(HorizontalAlignment::Right);
    p.set_is_first_paragraph_in_column(true);
    p.set_is_kept_with_next(true);
    p.set_is_in_new_page(true);
    p.set_is_in_line_paragraph(true);
    p.set_z_index(7);
    p.set_margin(MarginInfo::with_margins(1.0, 2.0, 3.0, 4.0));

    assert_eq!(p.vertical_alignment(), VerticalAlignment::Center);
    assert_eq!(p.horizontal_alignment(), HorizontalAlignment::Right);
    assert!(p.is_first_paragraph_in_column());
    assert!(p.is_kept_with_next());
    assert!(p.is_in_new_page());
    assert!(p.is_in_line_paragraph());
    assert_eq!(p.z_index(), 7);
    near(p.margin().left(), 1.0);
    near(p.margin().bottom(), 2.0);
    near(p.margin().right(), 3.0);
    near(p.margin().top(), 4.0);
}
