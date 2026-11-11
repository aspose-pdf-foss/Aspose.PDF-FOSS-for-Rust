//! `Aspose.Pdf.Matrix` — a 2-D affine transform in PDF user space,
//! per `capabilities/matrix.yaml`.
//!
//! The 6-tuple `[a b c d e f]` is the 3×3 affine matrix of PDF
//! 32000-1 §8.3 (row-vector convention `[x y 1] · M`). Arithmetic
//! delegates to the frozen `internal::transform` foundation rather
//! than being reimplemented here.
//!
//! Idiomatic adaptations of the canonical surface: the copy
//! constructor maps to derived `Clone` / `Copy`; `Reverse` and the
//! `UnScale` / `UnTransform` inverses return `Option` (`None` for a
//! singular matrix, mirroring the foundation's `inverse`); the
//! canonical `out`-parameter point mappers return tuples;
//! `Equals(Object)` maps to a typed [`Matrix::equals`] plus derived
//! `PartialEq` (as on [`Rectangle`] / [`Point`]); `ToString` and
//! `GetHashCode` are dropped.
//!
//! The `Rotation(f64)` factory is counter-clockwise (PDF §8.3.4,
//! matching the foundation) and takes **radians**; `from_rotation`
//! maps the enum quadrants in the same convention. `Skew` takes both
//! angles in **radians**. The static `Translate`
//! / `Scale` `(…, source)` factories post-compose onto `source`
//! (`source · translate`, `source · scale`); `GetFlipMatrix` returns
//! the vertical flip `(1, 0, 0, -1, 0, 0)`.

use crate::internal::transform;
use crate::public::point::Point;
use crate::public::rectangle::Rectangle;
use crate::public::rotation::Rotation;

/// `|det|` below this rejects a matrix from inversion — the same
/// singular threshold the foundation applies (PDF §8.3).
const SINGULAR_EPS: f64 = 1e-12;

/// A 2-D affine transform — the canonical `[a b c d e f]` tuple.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    inner: transform::Matrix,
}

impl Default for Matrix {
    /// The identity matrix — the canonical parameterless constructor.
    fn default() -> Self {
        Matrix {
            inner: transform::identity(),
        }
    }
}

impl Matrix {
    /// Construct from the six affine components `[a b c d e f]`.
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Matrix {
            inner: transform::Matrix::new(a, b, c, d, e, f),
        }
    }

    /// Construct from a `[a, b, c, d, e, f]` array (canonical
    /// `Matrix(double[])`; the `Data` element order).
    pub fn from_array(values: [f64; 6]) -> Self {
        Self::new(
            values[0], values[1], values[2], values[3], values[4], values[5],
        )
    }

    /// Construct from a single-precision `[a, b, c, d, e, f]` array
    /// (canonical `Matrix(float[])`; the `Elements` element order).
    pub fn from_elements(values: [f32; 6]) -> Self {
        Self::new(
            f64::from(values[0]),
            f64::from(values[1]),
            f64::from(values[2]),
            f64::from(values[3]),
            f64::from(values[4]),
            f64::from(values[5]),
        )
    }

    /// A counter-clockwise rotation matrix for `angle_radians`
    /// (canonical static `Rotation(double)`) — matches the foundation
    /// and PDF §8.3.4.
    pub fn rotation(angle_radians: f64) -> Matrix {
        Matrix {
            inner: transform::rotate(angle_radians),
        }
    }

    /// A counter-clockwise rotation matrix for a [`Rotation`]
    /// quadrant (canonical `Rotation(Rotation)`).
    pub fn from_rotation(rotation: Rotation) -> Matrix {
        let degrees: f64 = match rotation {
            Rotation::None => 0.0,
            Rotation::On90 => 90.0,
            Rotation::On180 => 180.0,
            Rotation::On270 => 270.0,
        };
        Self::rotation(degrees.to_radians())
    }

    /// A skew matrix for `alpha` (x-skew) and `beta` (y-skew), both
    /// in **radians** (canonical static `Skew`): `b = tan(alpha)`,
    /// `c = tan(beta)`.
    pub fn skew(alpha: f64, beta: f64) -> Matrix {
        Matrix {
            inner: transform::skew(alpha, beta),
        }
    }

    /// `source` translated by `(dx, dy)` — canonical static
    /// `Translate(dx, dy, source)` = `source · translate(dx, dy)`.
    pub fn translate(dx: f64, dy: f64, source: &Matrix) -> Matrix {
        Matrix {
            inner: transform::compose(source.inner, transform::translate(dx, dy)),
        }
    }

    /// `source` scaled by `(sx, sy)` — canonical static
    /// `Scale(sx, sy, source)` = `source · scale(sx, sy)`.
    pub fn scaling(sx: f64, sy: f64, source: &Matrix) -> Matrix {
        Matrix {
            inner: transform::compose(source.inner, transform::scale(sx, sy)),
        }
    }

    // --- components -------------------------------------------------

    /// The `a` component.
    pub fn a(&self) -> f64 {
        self.inner.a
    }
    /// Set the `a` component.
    pub fn set_a(&mut self, value: f64) {
        self.inner.a = value;
    }
    /// The `b` component.
    pub fn b(&self) -> f64 {
        self.inner.b
    }
    /// Set the `b` component.
    pub fn set_b(&mut self, value: f64) {
        self.inner.b = value;
    }
    /// The `c` component.
    pub fn c(&self) -> f64 {
        self.inner.c
    }
    /// Set the `c` component.
    pub fn set_c(&mut self, value: f64) {
        self.inner.c = value;
    }
    /// The `d` component.
    pub fn d(&self) -> f64 {
        self.inner.d
    }
    /// Set the `d` component.
    pub fn set_d(&mut self, value: f64) {
        self.inner.d = value;
    }
    /// The `e` component (x translation).
    pub fn e(&self) -> f64 {
        self.inner.e
    }
    /// Set the `e` component.
    pub fn set_e(&mut self, value: f64) {
        self.inner.e = value;
    }
    /// The `f` component (y translation).
    pub fn f(&self) -> f64 {
        self.inner.f
    }
    /// Set the `f` component.
    pub fn set_f(&mut self, value: f64) {
        self.inner.f = value;
    }

    /// The components as a `[a, b, c, d, e, f]` array (canonical
    /// `Data`).
    pub fn data(&self) -> [f64; 6] {
        [
            self.inner.a,
            self.inner.b,
            self.inner.c,
            self.inner.d,
            self.inner.e,
            self.inner.f,
        ]
    }

    /// The components as a single-precision array (canonical
    /// `Elements`).
    pub fn elements(&self) -> [f32; 6] {
        [
            self.inner.a as f32,
            self.inner.b as f32,
            self.inner.c as f32,
            self.inner.d as f32,
            self.inner.e as f32,
            self.inner.f as f32,
        ]
    }

    // --- arithmetic -------------------------------------------------

    /// `self · other` in PDF row-vector convention (canonical
    /// `Multiply`) — applying `self` then `other`.
    pub fn multiply(&self, other: &Matrix) -> Matrix {
        Matrix {
            inner: transform::compose(self.inner, other.inner),
        }
    }

    /// Component-wise sum (canonical `Add`).
    pub fn add(&self, other: &Matrix) -> Matrix {
        Matrix::new(
            self.inner.a + other.inner.a,
            self.inner.b + other.inner.b,
            self.inner.c + other.inner.c,
            self.inner.d + other.inner.d,
            self.inner.e + other.inner.e,
            self.inner.f + other.inner.f,
        )
    }

    /// The inverse transform (canonical `Reverse`). `None` when the
    /// matrix is singular (`|det| < 1e-12`).
    pub fn reverse(&self) -> Option<Matrix> {
        transform::inverse(self.inner).map(|m| Matrix { inner: m })
    }

    // --- point / rectangle mapping ---------------------------------

    /// Map a point through the full affine transform (canonical
    /// `Transform(Point)`).
    pub fn transform_point(&self, point: &Point) -> Point {
        let p = transform::apply(
            self.inner,
            transform::Point {
                x: point.x(),
                y: point.y(),
            },
        );
        Point::new(p.x, p.y)
    }

    /// The axis-aligned bounding box of the transformed rectangle
    /// (canonical `Transform(Rectangle)`).
    pub fn transform_rectangle(&self, rect: &Rectangle) -> Rectangle {
        let r = transform::apply_to_rect(
            self.inner,
            transform::Rect {
                x_min: rect.llx(),
                y_min: rect.lly(),
                x_max: rect.urx(),
                y_max: rect.ury(),
            },
        );
        Rectangle::new(r.x_min, r.y_min, r.x_max, r.y_max, false)
    }

    /// Apply the linear part only (no translation) to `(x, y)`,
    /// returning `(a·x + c·y, b·x + d·y)` — canonical `Scale`.
    pub fn scale(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.inner.a * x + self.inner.c * y,
            self.inner.b * x + self.inner.d * y,
        )
    }

    /// Invert [`Matrix::scale`] (canonical `UnScale`). `None` for a
    /// singular linear part.
    pub fn un_scale(&self, x1: f64, y1: f64) -> Option<(f64, f64)> {
        let det = self.inner.a * self.inner.d - self.inner.c * self.inner.b;
        if det.abs() < SINGULAR_EPS {
            return None;
        }
        Some((
            (self.inner.d * x1 - self.inner.c * y1) / det,
            (self.inner.a * y1 - self.inner.b * x1) / det,
        ))
    }

    /// Apply the full affine transform to `(x, y)`, returning
    /// `(a·x + c·y + e, b·x + d·y + f)` — canonical `Transform`.
    pub fn transform(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.inner.a * x + self.inner.c * y + self.inner.e,
            self.inner.b * x + self.inner.d * y + self.inner.f,
        )
    }

    /// Invert [`Matrix::transform`] (canonical `UnTransform`). `None`
    /// for a singular matrix.
    pub fn un_transform(&self, x1: f64, y1: f64) -> Option<(f64, f64)> {
        let det = self.inner.a * self.inner.d - self.inner.c * self.inner.b;
        if det.abs() < SINGULAR_EPS {
            return None;
        }
        let dx = x1 - self.inner.e;
        let dy = y1 - self.inner.f;
        Some((
            (self.inner.d * dx - self.inner.c * dy) / det,
            (self.inner.a * dy - self.inner.b * dx) / det,
        ))
    }

    /// The rotation quadrant as an angle in degrees (canonical
    /// `GetAngle`): `None → 0`, `On90 → 90`, `On180 → 180`,
    /// `On270 → 270`.
    pub fn get_angle(&self, rotation: Rotation) -> f64 {
        match rotation {
            Rotation::None => 0.0,
            Rotation::On90 => 90.0,
            Rotation::On180 => 180.0,
            Rotation::On270 => 270.0,
        }
    }

    /// The vertical-flip matrix `(1, 0, 0, -1, 0, 0)` (canonical
    /// `GetFlipMatrix`).
    pub fn get_flip_matrix(&self) -> Matrix {
        Matrix::new(1.0, 0.0, 0.0, -1.0, 0.0, 0.0)
    }

    /// Exact component-wise equality (canonical `Equals`).
    pub fn equals(&self, other: &Matrix) -> bool {
        self == other
    }
}
