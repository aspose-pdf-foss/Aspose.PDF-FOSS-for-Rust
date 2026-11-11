//! `Aspose.Pdf.Drawing.Line` — a poly-line shape, per
//! `capabilities/line.yaml`.

use crate::public::shape::sealed::ShapeRender;
use crate::public::shape::{fmt_num, impl_has_shape_base, Shape, ShapeBase};

/// A poly-line: a flat `[x0, y0, x1, y1, …]` point array.
#[derive(Debug, Default)]
pub struct Line {
    base: ShapeBase,
    position_array: Vec<f32>,
}

impl Line {
    /// Construct a line through the points in `position_array`
    /// (a flat `[x0, y0, x1, y1, …]` array).
    pub fn new(position_array: Vec<f32>) -> Self {
        Self { base: ShapeBase::default(), position_array }
    }

    /// The flat point array `[x0, y0, x1, y1, …]`.
    pub fn position_array(&self) -> &[f32] {
        &self.position_array
    }
    /// Set the flat point array.
    pub fn set_position_array(&mut self, value: Vec<f32>) {
        self.position_array = value;
    }
}

impl_has_shape_base!(Line);

impl ShapeRender for Line {
    fn render_path(&self, ops: &mut Vec<u8>) {
        let mut pts = self.position_array.chunks_exact(2);
        if let Some(first) = pts.next() {
            ops.extend_from_slice(
                format!("{} {} m\n", fmt_num(first[0] as f64), fmt_num(first[1] as f64)).as_bytes(),
            );
            for p in pts {
                ops.extend_from_slice(
                    format!("{} {} l\n", fmt_num(p[0] as f64), fmt_num(p[1] as f64)).as_bytes(),
                );
            }
        }
    }
}

impl Shape for Line {
    fn check_bounds(&self, container_width: f64, container_height: f64) -> bool {
        self.position_array.chunks_exact(2).all(|p| {
            let (x, y) = (p[0] as f64, p[1] as f64);
            (0.0..=container_width).contains(&x)
                && (0.0..=container_height).contains(&y)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_and_bounds() {
        let line = Line::new(vec![0.0, 0.0, 50.0, 50.0]);
        assert_eq!(line.position_array().len(), 4);
        assert_eq!(line.position_array()[2], 50.0);
        assert!(line.check_bounds(100.0, 100.0));
        assert!(!line.check_bounds(10.0, 10.0));
    }
}
