//! `Aspose.Pdf.Text.Position` — a text-placement coordinate, per
//! `capabilities/position.yaml`.
//!
//! A strict-subset value type carrying the `(XIndent, YIndent)`
//! point used by `TextFragment.Position` and `TextBuilder`. The
//! canonical `ToString` / `Equals` members are deferred.

/// A text-placement coordinate `(x_indent, y_indent)`, in points.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    x_indent: f64,
    y_indent: f64,
}

impl Position {
    /// Construct a position from horizontal and vertical indents.
    pub fn new(x_indent: f64, y_indent: f64) -> Self {
        Self { x_indent, y_indent }
    }

    /// The horizontal indent, in points.
    pub fn x_indent(&self) -> f64 {
        self.x_indent
    }
    /// Set the horizontal indent, in points.
    pub fn set_x_indent(&mut self, value: f64) {
        self.x_indent = value;
    }

    /// The vertical indent, in points.
    pub fn y_indent(&self) -> f64 {
        self.y_indent
    }
    /// Set the vertical indent, in points.
    pub fn set_y_indent(&mut self, value: f64) {
        self.y_indent = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_and_mutate() {
        let mut p = Position::new(72.0, 144.0);
        assert_eq!(p.x_indent(), 72.0);
        assert_eq!(p.y_indent(), 144.0);
        p.set_x_indent(10.0);
        p.set_y_indent(20.0);
        assert_eq!(p, Position::new(10.0, 20.0));
        assert_eq!(Position::default(), Position::new(0.0, 0.0));
    }
}
