// Public-API smoke driver — Margins.
//
// Coverage:
//   * Default ctor (new / Default::default) yields zero on all four
//     sides.
//   * with_sides round-trips each axis.
//   * Per-side setters round-trip and don't disturb the other sides.
//   * Copy + Eq semantics — Margins is a small POD value type.

use aspose_pdf_foss::Margins;

#[test]
fn default_ctor_all_zero() {
    let m = Margins::new();
    assert_eq!(m.left(), 0);
    assert_eq!(m.right(), 0);
    assert_eq!(m.top(), 0);
    assert_eq!(m.bottom(), 0);
}

#[test]
fn default_trait_matches_explicit_new() {
    let m: Margins = Default::default();
    assert_eq!(m, Margins::new());
}

#[test]
fn with_sides_round_trips() {
    let m = Margins::with_sides(1, 2, 3, 4);
    assert_eq!(m.left(), 1);
    assert_eq!(m.right(), 2);
    assert_eq!(m.top(), 3);
    assert_eq!(m.bottom(), 4);
}

#[test]
fn per_side_setters_round_trip() {
    let mut m = Margins::with_sides(1, 2, 3, 4);
    m.set_left(10);
    m.set_right(20);
    m.set_top(30);
    m.set_bottom(40);
    assert_eq!(m, Margins::with_sides(10, 20, 30, 40));
}

#[test]
fn setting_one_side_does_not_touch_others() {
    let mut m = Margins::with_sides(1, 2, 3, 4);
    m.set_top(999);
    assert_eq!(m.left(), 1);
    assert_eq!(m.right(), 2);
    assert_eq!(m.top(), 999);
    assert_eq!(m.bottom(), 4);
}

#[test]
fn negative_values_round_trip() {
    // The canonical surface is signed i32; negative-pixel margins
    // mean "bleed past the canvas edge". The carrier neither
    // validates nor clamps — it's the device's job to interpret.
    let m = Margins::with_sides(-5, -10, -15, -20);
    assert_eq!(m.left(), -5);
    assert_eq!(m.right(), -10);
    assert_eq!(m.top(), -15);
    assert_eq!(m.bottom(), -20);
}

#[test]
fn copy_semantics_independent_after_mutation() {
    let m1 = Margins::with_sides(1, 2, 3, 4);
    let mut m2 = m1;
    m2.set_left(99);
    assert_eq!(m1.left(), 1);
    assert_eq!(m2.left(), 99);
}
