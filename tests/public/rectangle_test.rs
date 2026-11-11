// Public-API smoke driver — Rectangle.
//
// Ports the cpp rectangle_smoke_test: corner / normalise ctor,
// width / height, equals / near_equals, intersect / join /
// is_intersect, contains family, center / to_points, rotate /
// move_by, and the empty / trivial / point predicates.

use aspose_pdf_foss::{Point, Rectangle, Rotation};

const EPS: f64 = 1e-9;
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < EPS, "{a} !~= {b}");
}

#[test]
fn ctor_and_normalise() {
    let r = Rectangle::new(10.0, 20.0, 30.0, 40.0, false);
    near(r.llx(), 10.0);
    near(r.lly(), 20.0);
    near(r.urx(), 30.0);
    near(r.ury(), 40.0);
    near(r.width(), 20.0);
    near(r.height(), 20.0);

    let n = Rectangle::new(30.0, 40.0, 10.0, 20.0, true);
    near(n.llx(), 10.0);
    near(n.lly(), 20.0);
    near(n.urx(), 30.0);
    near(n.ury(), 40.0);
}

#[test]
fn equals_and_near_equals() {
    let a = Rectangle::new(1.0, 2.0, 3.0, 4.0, false);
    let b = Rectangle::new(1.0, 2.0, 3.0, 4.0, false);
    let c = Rectangle::new(1.001, 2.0, 3.0, 4.0, false);
    assert!(a.equals(&b));
    assert!(!a.equals(&c));
    assert!(a.near_equals(&c, 0.01));
    assert!(!a.near_equals(&c, 0.0001));
}

#[test]
fn intersect_and_join() {
    let a = Rectangle::new(0.0, 0.0, 10.0, 10.0, false);
    let b = Rectangle::new(5.0, 5.0, 15.0, 15.0, false);
    let i = a.intersect(&b);
    near(i.llx(), 5.0);
    near(i.lly(), 5.0);
    near(i.urx(), 10.0);
    near(i.ury(), 10.0);
    let j = a.join(&b);
    near(j.llx(), 0.0);
    near(j.lly(), 0.0);
    near(j.urx(), 15.0);
    near(j.ury(), 15.0);
    assert!(a.is_intersect(&b));
    assert!(!a.is_intersect(&Rectangle::new(100.0, 100.0, 200.0, 200.0, false)));
}

#[test]
fn contains() {
    let r = Rectangle::new(0.0, 0.0, 10.0, 10.0, false);
    let inside = Point::new(5.0, 5.0);
    let on_edge = Point::new(10.0, 5.0);
    assert!(r.contains(&inside, true));
    assert!(r.contains(&inside, false));
    assert!(r.contains(&on_edge, true)); // inclusive
    assert!(!r.contains(&on_edge, false)); // exclusive
    assert!(r.contains_point(5.0, 5.0));
    assert!(r.contains_line(2.0, 2.0, 8.0, 8.0));
    assert!(!r.contains_line(2.0, 2.0, 20.0, 8.0));
}

#[test]
fn center_and_to_points() {
    let r = Rectangle::new(0.0, 0.0, 10.0, 20.0, false);
    let c = r.center();
    near(c.x(), 5.0);
    near(c.y(), 10.0);
    let pts = r.to_points();
    assert_eq!(pts.len(), 4);
    near(pts[0].x(), 0.0);
    near(pts[0].y(), 0.0);
    near(pts[2].x(), 10.0);
    near(pts[2].y(), 20.0);
}

#[test]
fn rotate_and_move_by() {
    let mut r = Rectangle::new(0.0, 0.0, 10.0, 20.0, false);
    r.rotate(Rotation::On90);
    near(r.width(), 20.0);
    near(r.height(), 10.0);

    let mut q = Rectangle::new(0.0, 0.0, 1.0, 1.0, false);
    q.move_by(3.0, 4.0);
    near(q.llx(), 3.0);
    near(q.lly(), 4.0);
    near(q.urx(), 4.0);
    near(q.ury(), 5.0);
}

#[test]
fn empty_trivial_point() {
    assert!(Rectangle::empty().is_empty());
    let t = Rectangle::trivial();
    assert!(t.is_trivial());
    assert!(t.is_point()); // origin-anchored zero-area
    let pt = Rectangle::new(5.0, 5.0, 5.0, 5.0, false);
    assert!(pt.is_point());
    assert!(!pt.is_trivial());
}
