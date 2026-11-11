//! `Aspose.Pdf.Drawing.Circle` — a circle shape, per
//! `capabilities/circle.yaml`.

use crate::public::shape::sealed::ShapeRender;
use crate::public::shape::{ellipse_path, impl_has_shape_base, Shape, ShapeBase};

/// A circle centred at `(pos_x, pos_y)` with a radius.
#[derive(Debug, Default)]
pub struct Circle {
    base: ShapeBase,
    pos_x: f64,
    pos_y: f64,
    radius: f64,
}

impl Circle {
    /// Construct a circle from its centre and radius.
    pub fn new(pos_x: f32, pos_y: f32, radius: f32) -> Self {
        Self {
            base: ShapeBase::default(),
            pos_x: pos_x as f64,
            pos_y: pos_y as f64,
            radius: radius as f64,
        }
    }

    /// The centre x coordinate, in points.
    pub fn pos_x(&self) -> f64 {
        self.pos_x
    }
    /// Set the centre x coordinate, in points.
    pub fn set_pos_x(&mut self, value: f64) {
        self.pos_x = value;
    }

    /// The centre y coordinate, in points.
    pub fn pos_y(&self) -> f64 {
        self.pos_y
    }
    /// Set the centre y coordinate, in points.
    pub fn set_pos_y(&mut self, value: f64) {
        self.pos_y = value;
    }

    /// The radius, in points.
    pub fn radius(&self) -> f64 {
        self.radius
    }
    /// Set the radius, in points.
    pub fn set_radius(&mut self, value: f64) {
        self.radius = value;
    }
}

impl_has_shape_base!(Circle);

impl ShapeRender for Circle {
    fn render_path(&self, ops: &mut Vec<u8>) {
        ellipse_path(ops, self.pos_x, self.pos_y, self.radius, self.radius);
    }
}

impl Shape for Circle {
    fn check_bounds(&self, container_width: f64, container_height: f64) -> bool {
        self.pos_x - self.radius >= 0.0
            && self.pos_y - self.radius >= 0.0
            && self.pos_x + self.radius <= container_width
            && self.pos_y + self.radius <= container_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_and_bounds() {
        let circle = Circle::new(50.0, 50.0, 20.0);
        assert_eq!(circle.radius(), 20.0);
        assert!(circle.check_bounds(100.0, 100.0));
        assert!(!circle.check_bounds(60.0, 60.0));
    }
}
