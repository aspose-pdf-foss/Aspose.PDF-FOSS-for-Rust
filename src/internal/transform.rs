// Pure 3×3 affine matrix arithmetic for PDF coordinate systems
// (PDF 32000-1:2008 §8.3). Matrices carry the 6-tuple wire layout
// (a, b, c, d, e, f); the third column is the affine invariant
// (0, 0, 1) and is folded back into arithmetic as needed. PDF uses
// row-vector convention: [x y 1] · M = [x' y' 1].

use std::fmt;

/// PDF affine matrix as a 6-tuple. Wire order: `[a b c d e f]`,
/// representing the 3×3
///
/// ```text
///   ⎡ a  b  0 ⎤
///   ⎢ c  d  0 ⎥
///   ⎣ e  f  1 ⎦
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x_min: f64,
    pub y_min: f64,
    pub x_max: f64,
    pub y_max: f64,
}

/// Errors returned by [`viewport`]. Other operations are total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// `rotate` was not in `{0, 90, 180, 270}` (PDF §14.8.4.4) or
    /// `pixel_width` / `pixel_height` was zero.
    InvalidArgument(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidArgument(msg) => write!(
                f, "transform: invalid argument: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl Matrix {
    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64)
        -> Self
    {
        Matrix { a, b, c, d, e, f }
    }
}

/// `|det| < SINGULAR_EPS` rejects a matrix from inversion. PDF §8.3
/// prescribes 1e-12 — small enough that real-world transforms
/// (det ≥ ~1e-6 for any sensible scaling) pass; large enough that
/// the inverse computation does not underflow.
const SINGULAR_EPS: f64 = 1e-12;

pub fn identity() -> Matrix {
    Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
}

#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn translate(tx: f64, ty: f64) -> Matrix {
    Matrix::new(1.0, 0.0, 0.0, 1.0, tx, ty)
}

#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn scale(sx: f64, sy: f64) -> Matrix {
    Matrix::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
}

/// Counter-clockwise rotation in user space. Row-vector convention
/// puts +sin θ at `b` and -sin θ at `c` — an implementer thinking
/// in column vectors will swap these and silently invert rotation.
#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn rotate(radians: f64) -> Matrix {
    let cos_t = radians.cos();
    let sin_t = radians.sin();
    Matrix::new(cos_t, sin_t, -sin_t, cos_t, 0.0, 0.0)
}

#[allow(dead_code)] // retained for a future-phase public-API binding
pub fn skew(alpha_x: f64, beta_y: f64) -> Matrix {
    Matrix::new(1.0, alpha_x.tan(), beta_y.tan(), 1.0, 0.0, 0.0)
}

/// `lhs · rhs` in row-vector convention (PDF §8.3.3). Associative
/// with [`identity`] as two-sided neutral; not commutative.
pub fn compose(lhs: Matrix, rhs: Matrix) -> Matrix {
    Matrix::new(
        lhs.a * rhs.a + lhs.b * rhs.c,
        lhs.a * rhs.b + lhs.b * rhs.d,
        lhs.c * rhs.a + lhs.d * rhs.c,
        lhs.c * rhs.b + lhs.d * rhs.d,
        lhs.e * rhs.a + lhs.f * rhs.c + rhs.e,
        lhs.e * rhs.b + lhs.f * rhs.d + rhs.f,
    )
}

/// Affine inverse. Returns `None` when `|det| < 1e-12` (matrix
/// singular or near-singular). Note `e' = (c*f - d*e) / det` —
/// flipping that sign is a known trap (composes-to-identity-on-
/// 2×2 but leaks (e, f); see semantic_traps in the spec yaml).
pub fn inverse(m: Matrix) -> Option<Matrix> {
    let det = m.a * m.d - m.b * m.c;
    if det.abs() < SINGULAR_EPS {
        return None;
    }
    Some(Matrix::new(
        m.d / det,
        -m.b / det,
        -m.c / det,
        m.a / det,
        (m.c * m.f - m.d * m.e) / det,
        (m.b * m.e - m.a * m.f) / det,
    ))
}

/// Apply `m` to point `p` via row-vector multiplication
/// `[x y 1] · M`. Restricted to first two output components
/// (third is invariant 1).
pub fn apply(m: Matrix, p: Point) -> Point {
    Point {
        x: m.a * p.x + m.c * p.y + m.e,
        y: m.b * p.x + m.d * p.y + m.f,
    }
}

/// Axis-aligned bounding box of the four transformed corners.
/// Strictly larger than `apply` of the diagonal under rotation
/// or shear — that's the documented behaviour of /BBox propagated
/// through a CTM.
// Used by page_renderer to place an annotation appearance; the
// standalone test mounts of this module do not reach it.
#[allow(dead_code)]
pub fn apply_to_rect(m: Matrix, r: Rect) -> Rect {
    let p1 = apply(m, Point { x: r.x_min, y: r.y_min });
    let p2 = apply(m, Point { x: r.x_max, y: r.y_min });
    let p3 = apply(m, Point { x: r.x_min, y: r.y_max });
    let p4 = apply(m, Point { x: r.x_max, y: r.y_max });
    Rect {
        x_min: p1.x.min(p2.x).min(p3.x).min(p4.x),
        y_min: p1.y.min(p2.y).min(p3.y).min(p4.y),
        x_max: p1.x.max(p2.x).max(p3.x).max(p4.x),
        y_max: p1.y.max(p2.y).max(p3.y).max(p4.y),
    }
}

/// Page-to-device viewport per PDF §8.3 Table 4 + §14.8.4.4.
/// `rotate` must be one of 0 / 90 / 180 / 270; pixel dims must be
/// non-zero. Closed-form per-/Rotate value derived from corner
/// mapping (mediabox top-left → device (0,0), bottom-right →
/// device (W, H)). The closed forms collapse the four-step
/// "translate-rotate-scale-translate" composition into a single
/// 6-tuple emit and are bit-identical (within eps 1e-12) to the
/// per-step path.
pub fn viewport(
    media_box: Rect,
    rotate_degrees: i32,
    pixel_width: u32,
    pixel_height: u32,
) -> Result<Matrix, Error> {
    if pixel_width == 0 {
        return Err(Error::InvalidArgument("pixel_width is zero"));
    }
    if pixel_height == 0 {
        return Err(Error::InvalidArgument("pixel_height is zero"));
    }
    let mx = media_box.x_min;
    let my = media_box.y_min;
    let mxx = media_box.x_max;
    let myy = media_box.y_max;
    let mw = mxx - mx;
    let mh = myy - my;
    let w = pixel_width as f64;
    let h = pixel_height as f64;
    match rotate_degrees {
        0 => {
            let sx = w / mw;
            let sy = h / mh;
            // x' = sx * (x - mx);  y' = sy * (MY - y) = -sy*y + sy*MY
            Ok(Matrix::new(sx, 0.0, 0.0, -sy, -sx * mx, sy * myy))
        }
        90 => {
            // Extents swapped: pixel canvas is (W=mh, H=mw).
            let sx = w / mh;
            let sy = h / mw;
            // x' = sx * (y - my);  y' = sy * (x - mx)
            Ok(Matrix::new(0.0, sy, sx, 0.0, -sx * my, -sy * mx))
        }
        180 => {
            let sx = w / mw;
            let sy = h / mh;
            // x' = -sx*x + sx*MX;  y' = sy*y - sy*my
            Ok(Matrix::new(-sx, 0.0, 0.0, sy, sx * mxx, -sy * my))
        }
        270 => {
            let sx = w / mh;
            let sy = h / mw;
            // x' = -sx*y + sx*MY;  y' = -sy*x + sy*MX
            Ok(Matrix::new(0.0, -sy, -sx, 0.0, sx * myy, sy * mxx))
        }
        _ => Err(Error::InvalidArgument(
            "rotate must be one of 0, 90, 180, 270")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    const EPS: f64 = 1e-12;

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    fn assert_matrix_close(actual: Matrix, expected: Matrix) {
        for (lhs, rhs) in [
            (actual.a, expected.a), (actual.b, expected.b),
            (actual.c, expected.c), (actual.d, expected.d),
            (actual.e, expected.e), (actual.f, expected.f),
        ] {
            assert!(approx_eq(lhs, rhs),
                    "matrix component {lhs} != {rhs}");
        }
    }

    #[test]
    fn identity_is_neutral() {
        let m = Matrix::new(2.0, 3.0, 4.0, 5.0, 6.0, 7.0);
        assert_matrix_close(compose(identity(), m), m);
        assert_matrix_close(compose(m, identity()), m);
    }

    #[test]
    fn translate_closed_form() {
        let m = translate(3.5, -7.25);
        assert_matrix_close(
            m,
            Matrix::new(1.0, 0.0, 0.0, 1.0, 3.5, -7.25));
    }

    #[test]
    fn scale_closed_form() {
        assert_matrix_close(
            scale(2.0, -3.0),
            Matrix::new(2.0, 0.0, 0.0, -3.0, 0.0, 0.0));
    }

    #[test]
    fn rotate_quarter_unit_x_to_unit_y() {
        // CCW rotation: (1, 0) → (0, 1).
        let m = rotate(PI / 2.0);
        let q = apply(m, Point { x: 1.0, y: 0.0 });
        assert!(approx_eq(q.x, 0.0));
        assert!(approx_eq(q.y, 1.0));
    }

    #[test]
    fn rotate_row_vector_sign_convention() {
        // b = +sin θ, c = -sin θ. An impl that swaps these
        // (column-vector convention) would still pass the unit-x
        // test under quarter rotation, but fails here at π/4.
        let m = rotate(PI / 4.0);
        let s = (PI / 4.0).sin();
        assert!(approx_eq(m.b, s));
        assert!(approx_eq(m.c, -s));
    }

    #[test]
    fn skew_closed_form() {
        let m = skew(0.5, 0.3);
        assert_matrix_close(
            m,
            Matrix::new(1.0, (0.5_f64).tan(),
                        (0.3_f64).tan(), 1.0, 0.0, 0.0));
    }

    #[test]
    fn compose_associative() {
        let m1 = translate(1.0, 2.0);
        let m2 = scale(3.0, 4.0);
        let m3 = rotate(PI / 6.0);
        let lhs = compose(compose(m1, m2), m3);
        let rhs = compose(m1, compose(m2, m3));
        assert_matrix_close(lhs, rhs);
    }

    #[test]
    fn compose_translate_then_rotate_not_commutative() {
        let t = translate(10.0, 0.0);
        let r = rotate(PI / 2.0);
        let tr = compose(t, r);
        let rt = compose(r, t);
        assert!(!approx_eq(tr.e, rt.e) || !approx_eq(tr.f, rt.f),
                "compose must not be commutative");
    }

    #[test]
    fn inverse_undoes_translate() {
        let m = translate(7.0, -3.0);
        let inv = inverse(m).unwrap();
        let p = apply(compose(m, inv), Point { x: 5.0, y: 5.0 });
        assert!(approx_eq(p.x, 5.0));
        assert!(approx_eq(p.y, 5.0));
    }

    #[test]
    fn inverse_undoes_general_matrix() {
        let m = Matrix::new(2.0, 1.0, -1.0, 3.0, 4.0, 5.0);
        let inv = inverse(m).unwrap();
        let composed = compose(m, inv);
        assert_matrix_close(composed, identity());
    }

    #[test]
    fn inverse_singular_returns_none() {
        // det = 0 — both rows of the 2x2 sub-block are zero.
        let m = Matrix::new(0.0, 0.0, 0.0, 0.0, 5.0, 7.0);
        assert!(inverse(m).is_none());
    }

    #[test]
    fn inverse_near_singular_returns_none() {
        // det = 1e-13, below the 1e-12 threshold.
        let m = Matrix::new(1e-7, 0.0, 0.0, 1e-6, 0.0, 0.0);
        assert!(inverse(m).is_none());
    }

    #[test]
    fn apply_translate() {
        let q = apply(translate(3.0, 4.0), Point { x: 1.0, y: 2.0 });
        assert!(approx_eq(q.x, 4.0));
        assert!(approx_eq(q.y, 6.0));
    }

    #[test]
    fn apply_to_rect_axis_aligned() {
        let r = Rect { x_min: 0.0, y_min: 0.0,
                       x_max: 1.0, y_max: 1.0 };
        let bbox = apply_to_rect(translate(5.0, 7.0), r);
        assert!(approx_eq(bbox.x_min, 5.0));
        assert!(approx_eq(bbox.y_min, 7.0));
        assert!(approx_eq(bbox.x_max, 6.0));
        assert!(approx_eq(bbox.y_max, 8.0));
    }

    #[test]
    fn apply_to_rect_under_rotation_strictly_enlarges() {
        // 45° rotation of unit square — diagonal endpoints would
        // give a 1×1 bbox; correct answer is √2 × √2 ≈ 1.41.
        let r = Rect { x_min: 0.0, y_min: 0.0,
                       x_max: 1.0, y_max: 1.0 };
        let bbox = apply_to_rect(rotate(PI / 4.0), r);
        let span_x = bbox.x_max - bbox.x_min;
        let span_y = bbox.y_max - bbox.y_min;
        assert!(approx_eq(span_x, 2.0_f64.sqrt()));
        assert!(approx_eq(span_y, 2.0_f64.sqrt()));
    }

    #[test]
    fn viewport_letter_rotate_0() {
        let mb = Rect { x_min: 0.0, y_min: 0.0,
                        x_max: 612.0, y_max: 792.0 };
        let m = viewport(mb, 0, 612, 792).unwrap();
        assert_matrix_close(
            m,
            Matrix::new(1.0, 0.0, 0.0, -1.0, 0.0, 792.0));
    }

    #[test]
    fn viewport_letter_rotate_90() {
        let mb = Rect { x_min: 0.0, y_min: 0.0,
                        x_max: 612.0, y_max: 792.0 };
        let m = viewport(mb, 90, 792, 612).unwrap();
        assert_matrix_close(
            m,
            Matrix::new(0.0, 1.0, 1.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn viewport_letter_rotate_180() {
        let mb = Rect { x_min: 0.0, y_min: 0.0,
                        x_max: 612.0, y_max: 792.0 };
        let m = viewport(mb, 180, 612, 792).unwrap();
        assert_matrix_close(
            m,
            Matrix::new(-1.0, 0.0, 0.0, 1.0, 612.0, 0.0));
    }

    #[test]
    fn viewport_letter_rotate_270() {
        let mb = Rect { x_min: 0.0, y_min: 0.0,
                        x_max: 612.0, y_max: 792.0 };
        let m = viewport(mb, 270, 792, 612).unwrap();
        assert_matrix_close(
            m,
            Matrix::new(0.0, -1.0, -1.0, 0.0, 792.0, 612.0));
    }

    #[test]
    fn viewport_rejects_bad_rotate() {
        let mb = Rect { x_min: 0.0, y_min: 0.0,
                        x_max: 1.0, y_max: 1.0 };
        assert!(viewport(mb, 45, 100, 100).is_err());
        assert!(viewport(mb, -90, 100, 100).is_err());
    }

    #[test]
    fn viewport_rejects_zero_pixel_dim() {
        let mb = Rect { x_min: 0.0, y_min: 0.0,
                        x_max: 1.0, y_max: 1.0 };
        assert!(viewport(mb, 0, 0, 100).is_err());
        assert!(viewport(mb, 0, 100, 0).is_err());
    }
}
