//! `Aspose.Pdf.Annotations.FixedPrint` — the fixed-print placement of a
//! watermark annotation, per `capabilities/fixed_print.yaml`.
//!
//! Controls how a watermark is positioned when the page is printed: a
//! [`matrix`](FixedPrint::matrix) transform plus a horizontal / vertical
//! translation (in points). Reached via
//! [`WatermarkAnnotation::fixed_print`](crate::WatermarkAnnotation::fixed_print).
//!
//! **Carried, not written**, and not for want of a place to put it:
//! §12.5.6.22's `/FixedPrint` says where to place the annotation's
//! *appearance* when printing, and a watermark here is written as a
//! dictionary with no appearance stream — so the dictionary would place
//! nothing. Writing an entry that provably takes no effect would say
//! something untrue. See
//! [`WatermarkAnnotation`](crate::WatermarkAnnotation), where the same
//! call is recorded for the watermark's own text and opacity.

use crate::public::matrix::Matrix;

/// The fixed-print placement of a watermark annotation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FixedPrint {
    matrix: Matrix,
    horizontal_translation: f64,
    vertical_translation: f64,
}

impl FixedPrint {
    /// Construct with the identity matrix and zero translation.
    pub fn new() -> Self {
        Self::default()
    }

    /// The fixed-print transform matrix.
    pub fn matrix(&self) -> Matrix {
        self.matrix
    }
    /// Set the fixed-print transform matrix.
    pub fn set_matrix(&mut self, value: Matrix) {
        self.matrix = value;
    }

    /// The horizontal translation, in points.
    pub fn horizontal_translation(&self) -> f64 {
        self.horizontal_translation
    }
    /// Set the horizontal translation, in points.
    pub fn set_horizontal_translation(&mut self, value: f64) {
        self.horizontal_translation = value;
    }

    /// The vertical translation, in points.
    pub fn vertical_translation(&self) -> f64 {
        self.vertical_translation
    }
    /// Set the vertical translation, in points.
    pub fn set_vertical_translation(&mut self, value: f64) {
        self.vertical_translation = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_setters() {
        let mut fp = FixedPrint::new();
        assert_eq!(fp.horizontal_translation(), 0.0);
        assert_eq!(fp.vertical_translation(), 0.0);
        assert_eq!(fp.matrix(), Matrix::default());
        fp.set_horizontal_translation(12.0);
        fp.set_vertical_translation(-4.0);
        fp.set_matrix(Matrix::new(1.0, 0.0, 0.0, 1.0, 5.0, 6.0));
        assert_eq!(fp.horizontal_translation(), 12.0);
        assert_eq!(fp.vertical_translation(), -4.0);
        assert_eq!(fp.matrix(), Matrix::new(1.0, 0.0, 0.0, 1.0, 5.0, 6.0));
    }
}
