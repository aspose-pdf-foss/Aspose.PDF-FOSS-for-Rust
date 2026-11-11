//! `Aspose.Pdf.Point` — a 2-D point in PDF user space (double
//! precision), per `capabilities/point.yaml`.
//!
//! Mirrors the canonical member selection the C++ reference ships:
//! the two-arg constructor, read-write `X` / `Y` accessors, the
//! static `Distance` between two points, and the static `Trivial`
//! origin point. The canonical `ToPoint` (System.Drawing.PointF)
//! and `ToString` members are dropped — `PointF` has no Rust
//! equivalent and the reference omits the string rendering.

/// A point in PDF user space, carrying `x` / `y` coordinates in
/// double precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    /// Construct a point from explicit `x` and `y` coordinates.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The x coordinate.
    pub fn x(&self) -> f64 {
        self.x
    }

    /// Replace the x coordinate. See [`Point::x`].
    pub fn set_x(&mut self, value: f64) {
        self.x = value;
    }

    /// The y coordinate.
    pub fn y(&self) -> f64 {
        self.y
    }

    /// Replace the y coordinate. See [`Point::y`].
    pub fn set_y(&mut self, value: f64) {
        self.y = value;
    }

    /// The Euclidean distance between `point1` and `point2`.
    ///
    /// Canonical static `Aspose.Pdf.Point.Distance`.
    pub fn distance(point1: &Point, point2: &Point) -> f64 {
        let dx = point1.x - point2.x;
        let dy = point1.y - point2.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// The trivial point at the origin (`0, 0`).
    ///
    /// Canonical static `Aspose.Pdf.Point.Trivial`.
    pub fn trivial() -> Point {
        Point { x: 0.0, y: 0.0 }
    }
}
