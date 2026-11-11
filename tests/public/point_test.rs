// Public-API smoke driver — Point.
//
// Coverage:
//   * Two-arg ctor stores x / y.
//   * Read-write accessors round-trip each axis.
//   * Static Distance computes the Euclidean distance.
//   * Static Trivial yields the origin.
//   * Copy + PartialEq semantics — Point is a small POD value type.

use aspose_pdf_foss::Point;

#[test]
fn ctor_stores_coordinates() {
    let p = Point::new(3.0, 4.0);
    assert_eq!(p.x(), 3.0);
    assert_eq!(p.y(), 4.0);
}

#[test]
fn accessors_round_trip() {
    let mut p = Point::new(1.0, 2.0);
    p.set_x(10.0);
    p.set_y(20.0);
    assert_eq!(p.x(), 10.0);
    assert_eq!(p.y(), 20.0);
}

#[test]
fn distance_is_euclidean() {
    // 3-4-5 right triangle.
    let a = Point::new(0.0, 0.0);
    let b = Point::new(3.0, 4.0);
    assert_eq!(Point::distance(&a, &b), 5.0);
    // Symmetric.
    assert_eq!(Point::distance(&b, &a), 5.0);
    // Zero distance to self.
    assert_eq!(Point::distance(&a, &a), 0.0);
}

#[test]
fn trivial_is_origin() {
    let t = Point::trivial();
    assert_eq!(t.x(), 0.0);
    assert_eq!(t.y(), 0.0);
}

#[test]
fn copy_and_eq() {
    let p = Point::new(5.5, -2.5);
    let q = p;
    assert_eq!(p, q);
    assert_eq!(p.x(), 5.5);
}
