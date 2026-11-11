//! `Aspose.Pdf.Drawing.Ellipse` — an ellipse shape, per
//! `capabilities/ellipse.yaml`.

use crate::public::shape::sealed::ShapeRender;
use crate::public::shape::{ellipse_path, impl_has_shape_base, Shape, ShapeBase};

/// An ellipse inscribed in the box `(left, bottom, width, height)`.
#[derive(Debug, Default)]
pub struct Ellipse {
    base: ShapeBase,
    left: f64,
    bottom: f64,
    width: f64,
    height: f64,
}

impl Ellipse {
    /// Construct an ellipse from its bounding box.
    pub fn new(left: f64, bottom: f64, width: f64, height: f64) -> Self {
        Self { base: ShapeBase::default(), left, bottom, width, height }
    }

    /// The left edge, in points.
    pub fn left(&self) -> f64 {
        self.left
    }
    /// Set the left edge, in points.
    pub fn set_left(&mut self, value: f64) {
        self.left = value;
    }

    /// The bottom edge, in points.
    pub fn bottom(&self) -> f64 {
        self.bottom
    }
    /// Set the bottom edge, in points.
    pub fn set_bottom(&mut self, value: f64) {
        self.bottom = value;
    }

    /// The width, in points.
    pub fn width(&self) -> f64 {
        self.width
    }
    /// Set the width, in points.
    pub fn set_width(&mut self, value: f64) {
        self.width = value;
    }

    /// The height, in points.
    pub fn height(&self) -> f64 {
        self.height
    }
    /// Set the height, in points.
    pub fn set_height(&mut self, value: f64) {
        self.height = value;
    }
}

impl_has_shape_base!(Ellipse);

impl ShapeRender for Ellipse {
    fn render_path(&self, ops: &mut Vec<u8>) {
        let rx = self.width / 2.0;
        let ry = self.height / 2.0;
        ellipse_path(ops, self.left + rx, self.bottom + ry, rx, ry);
    }
}

impl Shape for Ellipse {
    fn check_bounds(&self, container_width: f64, container_height: f64) -> bool {
        self.left >= 0.0
            && self.bottom >= 0.0
            && self.left + self.width <= container_width
            && self.bottom + self.height <= container_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_and_accessors() {
        let ell = Ellipse::new(10.0, 10.0, 30.0, 20.0);
        assert_eq!(ell.height(), 20.0);
        assert!(ell.check_bounds(100.0, 100.0));
    }
}
