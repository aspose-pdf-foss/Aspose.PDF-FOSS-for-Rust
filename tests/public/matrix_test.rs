// Public-API smoke driver — Matrix (affine-transform core).
//
// Binds Aspose.Pdf.Matrix to the frozen `transform` foundation. The
// rotation/skew factories and GetFlipMatrix are deferred pending
// canonical-semantics confirmation and are not exercised here.

use aspose_pdf_foss::{Matrix, Point, Rectangle, Rotation};

const EPS: f64 = 1e-12;

#[test]
fn default_is_identity_and_neutral() {
    let id = Matrix::default();
    assert_eq!(id.data(), [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let m = Matrix::new(2.0, 3.0, 4.0, 5.0, 6.0, 7.0);
    assert_eq!(id.multiply(&m), m);
    assert_eq!(m.multiply(&id), m);
}

#[test]
fn components_round_trip_through_arrays() {
    let mut m = Matrix::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    assert_eq!(
        (m.a(), m.b(), m.c(), m.d(), m.e(), m.f()),
        (1.0, 2.0, 3.0, 4.0, 5.0, 6.0)
    );
    m.set_e(10.0);
    assert_eq!(m.e(), 10.0);
    assert_eq!(Matrix::from_array(m.data()), m);
    // Components are small exact integers — lossless across f32.
    assert_eq!(Matrix::from_elements(m.elements()), m);
}

#[test]
fn transform_applies_translation_scale_omits_it() {
    let m = Matrix::new(1.0, 0.0, 0.0, 1.0, 5.0, 7.0); // translate (5, 7)
    let p = m.transform_point(&Point::new(1.0, 2.0));
    assert_eq!((p.x(), p.y()), (6.0, 9.0));
    assert_eq!(m.transform(1.0, 2.0), (6.0, 9.0));
    assert_eq!(m.scale(1.0, 2.0), (1.0, 2.0)); // linear part only
}

#[test]
fn reverse_and_un_transform_round_trip() {
    let m = Matrix::new(2.0, 1.0, -1.0, 3.0, 4.0, 5.0);
    let inv = m.reverse().expect("non-singular");
    for (got, want) in m.multiply(&inv).data().iter().zip([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]) {
        assert!((got - want).abs() < EPS, "{got} != {want}");
    }
    let (x1, y1) = m.transform(3.0, 4.0);
    let (x, y) = m.un_transform(x1, y1).expect("non-singular");
    assert!((x - 3.0).abs() < EPS && (y - 4.0).abs() < EPS);
}

#[test]
fn singular_matrix_has_no_inverse() {
    let singular = Matrix::new(0.0, 0.0, 0.0, 0.0, 5.0, 7.0);
    assert!(singular.reverse().is_none());
    assert!(singular.un_scale(1.0, 1.0).is_none());
    assert!(singular.un_transform(1.0, 1.0).is_none());
}

#[test]
fn transform_rectangle_returns_bounding_box() {
    let m = Matrix::new(1.0, 0.0, 0.0, 1.0, 10.0, 20.0); // translate
    let r = m.transform_rectangle(&Rectangle::new(0.0, 0.0, 2.0, 3.0, false));
    assert_eq!((r.llx(), r.lly(), r.urx(), r.ury()), (10.0, 20.0, 12.0, 23.0));
}

#[test]
fn get_angle_maps_rotation_quadrants_to_degrees() {
    let m = Matrix::default();
    assert_eq!(m.get_angle(Rotation::None), 0.0);
    assert_eq!(m.get_angle(Rotation::On90), 90.0);
    assert_eq!(m.get_angle(Rotation::On180), 180.0);
    assert_eq!(m.get_angle(Rotation::On270), 270.0);
}

#[test]
fn rotation_is_counter_clockwise_and_in_radians() {
    use std::f64::consts::FRAC_PI_2;
    // CCW 90° (π/2 rad) in PDF y-up space sends +x (1, 0) to (0, 1).
    let p = Matrix::rotation(FRAC_PI_2).transform_point(&Point::new(1.0, 0.0));
    assert!(
        p.x().abs() < EPS && (p.y() - 1.0).abs() < EPS,
        "ccw π/2: got ({}, {})",
        p.x(),
        p.y()
    );
    // rotation(0) is the identity; the enum quadrant agrees with the
    // radian factory.
    assert_eq!(Matrix::rotation(0.0), Matrix::default());
    assert_eq!(
        Matrix::from_rotation(Rotation::On90),
        Matrix::rotation(FRAC_PI_2)
    );
}

#[test]
fn skew_builds_tangent_shear_in_radians() {
    let d = Matrix::skew(0.5, 0.3).data(); // [a b c d e f]
    assert!((d[0] - 1.0).abs() < EPS && (d[3] - 1.0).abs() < EPS);
    assert!((d[1] - 0.5_f64.tan()).abs() < EPS && (d[2] - 0.3_f64.tan()).abs() < EPS);
    assert_eq!((d[4], d[5]), (0.0, 0.0));
}

#[test]
fn static_factories_post_compose_onto_source() {
    let src = Matrix::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0); // scale ×2
    // Translate(dx,dy,source) == source · translate(dx,dy)
    let translated = Matrix::translate(5.0, 7.0, &src);
    assert_eq!(translated, src.multiply(&Matrix::new(1.0, 0.0, 0.0, 1.0, 5.0, 7.0)));
    // Scale(sx,sy,source) == source · scale(sx,sy)
    let scaled = Matrix::scaling(3.0, 4.0, &src);
    assert_eq!(scaled, src.multiply(&Matrix::new(3.0, 0.0, 0.0, 4.0, 0.0, 0.0)));
}

#[test]
fn flip_matrix_is_vertical() {
    let flip = Matrix::default().get_flip_matrix();
    assert_eq!(flip.data(), [1.0, 0.0, 0.0, -1.0, 0.0, 0.0]);
}
