//! `Aspose.Pdf.Drawing.Rectangle` — a rectangle shape, per
//! `capabilities/drawing_rectangle.yaml`.
//!
//! Exported as `DrawingRectangle` to avoid clashing with the
//! geometry [`Rectangle`](crate::Rectangle).

use crate::public::shape::sealed::ShapeRender;
use crate::public::shape::{fmt_num, impl_has_shape_base, Shape, ShapeBase};

/// A rectangle shape `(left, bottom, width, height)` with an
/// optional rounded corner radius.
#[derive(Debug, Default)]
pub struct DrawingRectangle {
    base: ShapeBase,
    left: f64,
    bottom: f64,
    width: f64,
    height: f64,
    rounded_corner_radius: f64,
}

impl DrawingRectangle {
    /// Construct a rectangle from its lower-left corner and size.
    pub fn new(left: f32, bottom: f32, width: f32, height: f32) -> Self {
        Self {
            base: ShapeBase::default(),
            left: left as f64,
            bottom: bottom as f64,
            width: width as f64,
            height: height as f64,
            rounded_corner_radius: 0.0,
        }
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

    /// The corner radius, in points.
    pub fn rounded_corner_radius(&self) -> f64 {
        self.rounded_corner_radius
    }
    /// Set the corner radius, in points.
    pub fn set_rounded_corner_radius(&mut self, value: f64) {
        self.rounded_corner_radius = value;
    }
}

impl_has_shape_base!(DrawingRectangle);

impl ShapeRender for DrawingRectangle {
    fn render_path(&self, ops: &mut Vec<u8>) {
        ops.extend_from_slice(
            format!(
                "{} {} {} {} re\n",
                fmt_num(self.left),
                fmt_num(self.bottom),
                fmt_num(self.width),
                fmt_num(self.height)
            )
            .as_bytes(),
        );
    }
}

impl Shape for DrawingRectangle {
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
    fn construct_and_bounds() {
        let mut rect = DrawingRectangle::new(5.0, 5.0, 40.0, 30.0);
        assert_eq!(rect.width(), 40.0);
        rect.set_rounded_corner_radius(3.0);
        assert_eq!(rect.rounded_corner_radius(), 3.0);
        assert!(rect.check_bounds(100.0, 100.0));
    }
}
