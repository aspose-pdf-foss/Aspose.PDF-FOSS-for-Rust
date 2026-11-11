//! `Aspose.Pdf.Rectangle` — an axis-aligned rectangle in PDF user
//! space, per `capabilities/rectangle.yaml`.
//!
//! Coordinates are the lower-left (`llx`, `lly`) and upper-right
//! (`urx`, `ury`) corners. The C++ reference's geometry surface is
//! mirrored; the `System.Drawing.Rectangle` bridges (`ToRect` /
//! `FromRect`), `ToString` / `Parse`, and `Clone` are dropped (see
//! `targets/rust/{translations,drops}.yaml`).

use crate::public::point::Point;
use crate::public::rotation::Rotation;

/// An axis-aligned rectangle (lower-left + upper-right corners).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    llx: f64,
    lly: f64,
    urx: f64,
    ury: f64,
}

impl Rectangle {
    /// Construct from the four edge coordinates. When
    /// `normalize_coordinates` is set, the corners are sorted so
    /// `llx <= urx` and `lly <= ury`.
    pub fn new(llx: f64, lly: f64, urx: f64, ury: f64, normalize_coordinates: bool) -> Self {
        if normalize_coordinates {
            Self {
                llx: llx.min(urx),
                lly: lly.min(ury),
                urx: llx.max(urx),
                ury: lly.max(ury),
            }
        } else {
            Self { llx, lly, urx, ury }
        }
    }

    /// The lower-left x coordinate.
    pub fn llx(&self) -> f64 {
        self.llx
    }
    /// Set the lower-left x coordinate.
    pub fn set_llx(&mut self, value: f64) {
        self.llx = value;
    }
    /// The lower-left y coordinate.
    pub fn lly(&self) -> f64 {
        self.lly
    }
    /// Set the lower-left y coordinate.
    pub fn set_lly(&mut self, value: f64) {
        self.lly = value;
    }
    /// The upper-right x coordinate.
    pub fn urx(&self) -> f64 {
        self.urx
    }
    /// Set the upper-right x coordinate.
    pub fn set_urx(&mut self, value: f64) {
        self.urx = value;
    }
    /// The upper-right y coordinate.
    pub fn ury(&self) -> f64 {
        self.ury
    }
    /// Set the upper-right y coordinate.
    pub fn set_ury(&mut self, value: f64) {
        self.ury = value;
    }

    /// The width (`urx - llx`).
    pub fn width(&self) -> f64 {
        self.urx - self.llx
    }
    /// The height (`ury - lly`).
    pub fn height(&self) -> f64 {
        self.ury - self.lly
    }

    /// Exact corner-by-corner equality.
    pub fn equals(&self, other: &Rectangle) -> bool {
        self == other
    }

    /// Whether every corner is within `epsilon` of `other`'s.
    pub fn near_equals(&self, other: &Rectangle, epsilon: f64) -> bool {
        (self.llx - other.llx).abs() <= epsilon
            && (self.lly - other.lly).abs() <= epsilon
            && (self.urx - other.urx).abs() <= epsilon
            && (self.ury - other.ury).abs() <= epsilon
    }

    /// The intersection (overlap) of this rectangle and `other`.
    pub fn intersect(&self, other: &Rectangle) -> Rectangle {
        Rectangle {
            llx: self.llx.max(other.llx),
            lly: self.lly.max(other.lly),
            urx: self.urx.min(other.urx),
            ury: self.ury.min(other.ury),
        }
    }

    /// The smallest rectangle containing both this and `other`.
    pub fn join(&self, other: &Rectangle) -> Rectangle {
        Rectangle {
            llx: self.llx.min(other.llx),
            lly: self.lly.min(other.lly),
            urx: self.urx.max(other.urx),
            ury: self.ury.max(other.ury),
        }
    }

    /// Whether this rectangle overlaps `other` (positive area).
    pub fn is_intersect(&self, other: &Rectangle) -> bool {
        self.llx < other.urx
            && other.llx < self.urx
            && self.lly < other.ury
            && other.lly < self.ury
    }

    /// Whether `point` lies within the rectangle. `include_border`
    /// selects inclusive (`<=`) vs exclusive (`<`) edge handling.
    pub fn contains(&self, point: &Point, include_border: bool) -> bool {
        self.contains_xy(point.x(), point.y(), include_border)
    }

    /// Whether the point `(x, y)` lies within the rectangle
    /// (inclusive of the border).
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        self.contains_xy(x, y, true)
    }

    /// Whether the segment `(x1, y1)`–`(x2, y2)` lies entirely
    /// within the rectangle (inclusive of the border).
    pub fn contains_line(&self, x1: f64, y1: f64, x2: f64, y2: f64) -> bool {
        self.contains_xy(x1, y1, true) && self.contains_xy(x2, y2, true)
    }

    fn contains_xy(&self, x: f64, y: f64, include_border: bool) -> bool {
        if include_border {
            x >= self.llx && x <= self.urx && y >= self.lly && y <= self.ury
        } else {
            x > self.llx && x < self.urx && y > self.lly && y < self.ury
        }
    }

    /// The centre point.
    pub fn center(&self) -> Point {
        Point::new((self.llx + self.urx) / 2.0, (self.lly + self.ury) / 2.0)
    }

    /// The four corners, counter-clockwise from the lower-left:
    /// `[LL, LR, UR, UL]`.
    pub fn to_points(&self) -> Vec<Point> {
        vec![
            Point::new(self.llx, self.lly),
            Point::new(self.urx, self.lly),
            Point::new(self.urx, self.ury),
            Point::new(self.llx, self.ury),
        ]
    }

    /// Translate the rectangle by `(dx, dy)`.
    pub fn move_by(&mut self, dx: f64, dy: f64) {
        self.llx += dx;
        self.urx += dx;
        self.lly += dy;
        self.ury += dy;
    }

    /// Rotate by a [`Rotation`] quadrant, anchored at the lower-left
    /// corner. 90° / 270° swap width and height.
    pub fn rotate(&mut self, angle: Rotation) {
        self.rotate_degrees(angle as i32 * 90);
    }

    /// Rotate by an angle in degrees (normalised to the nearest
    /// quadrant), anchored at the lower-left corner.
    pub fn rotate_by(&mut self, angle: i32) {
        self.rotate_degrees(angle);
    }

    fn rotate_degrees(&mut self, degrees: i32) {
        let quadrant = degrees.rem_euclid(360) / 90;
        if quadrant == 1 || quadrant == 3 {
            let (w, h) = (self.width(), self.height());
            self.urx = self.llx + h;
            self.ury = self.lly + w;
        }
    }

    /// The empty rectangle — an inverted sentinel (`llx > urx`) for
    /// which [`is_empty`](Self::is_empty) holds.
    pub fn empty() -> Rectangle {
        Rectangle {
            llx: f64::INFINITY,
            lly: f64::INFINITY,
            urx: f64::NEG_INFINITY,
            ury: f64::NEG_INFINITY,
        }
    }

    /// The trivial rectangle — the origin-anchored zero rectangle
    /// `(0, 0, 0, 0)`.
    pub fn trivial() -> Rectangle {
        Rectangle {
            llx: 0.0,
            lly: 0.0,
            urx: 0.0,
            ury: 0.0,
        }
    }

    /// Whether this is the trivial (all-zero) rectangle.
    pub fn is_trivial(&self) -> bool {
        self.llx == 0.0 && self.lly == 0.0 && self.urx == 0.0 && self.ury == 0.0
    }

    /// Whether this rectangle is empty (inverted: `llx > urx` or
    /// `lly > ury`).
    pub fn is_empty(&self) -> bool {
        self.llx > self.urx || self.lly > self.ury
    }

    /// Whether this rectangle has zero area (a single point).
    pub fn is_point(&self) -> bool {
        self.llx == self.urx && self.lly == self.ury
    }
}
