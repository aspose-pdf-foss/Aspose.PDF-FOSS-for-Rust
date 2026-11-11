//! `Aspose.Pdf.MarginInfo` — paragraph / box margins, per
//! `capabilities/margin_info.yaml`.
//!
//! A four-edge inset value type (left / bottom / right / top, in
//! points). The canonical `ICloneable.Clone` (returns
//! `System.Object`) is dropped in favour of Rust's derived `Clone`.
//!
//! **Carried, not applied.** Nothing reads an edge: the layout layer
//! that would inset by one — a paragraph's or a cell's margin — renders
//! from an origin it is given rather than from a box it owns, which is
//! the shared reason recorded at `RenderedParagraph` in
//! [`BaseParagraph`](crate::BaseParagraph). All four edges store and
//! read back exactly. [`Margins`](crate::Margins), the device-side
//! carrier in pixels, is unapplied for a reason of its own.

/// Margins (insets) on each edge, in points.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MarginInfo {
    left: f64,
    bottom: f64,
    right: f64,
    top: f64,
}

impl MarginInfo {
    /// Construct zero margins.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct margins from the four edge insets, in points.
    /// Argument order matches the canonical constructor:
    /// `left, bottom, right, top`.
    pub fn with_margins(left: f64, bottom: f64, right: f64, top: f64) -> Self {
        Self { left, bottom, right, top }
    }

    /// The left inset, in points.
    pub fn left(&self) -> f64 {
        self.left
    }
    /// Set the left inset, in points.
    pub fn set_left(&mut self, value: f64) {
        self.left = value;
    }

    /// The right inset, in points.
    pub fn right(&self) -> f64 {
        self.right
    }
    /// Set the right inset, in points.
    pub fn set_right(&mut self, value: f64) {
        self.right = value;
    }

    /// The top inset, in points.
    pub fn top(&self) -> f64 {
        self.top
    }
    /// Set the top inset, in points.
    pub fn set_top(&mut self, value: f64) {
        self.top = value;
    }

    /// The bottom inset, in points.
    pub fn bottom(&self) -> f64 {
        self.bottom
    }
    /// Set the bottom inset, in points.
    pub fn set_bottom(&mut self, value: f64) {
        self.bottom = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_and_mutate() {
        let mut m = MarginInfo::with_margins(1.0, 2.0, 3.0, 4.0);
        assert_eq!((m.left(), m.bottom(), m.right(), m.top()), (1.0, 2.0, 3.0, 4.0));
        m.set_left(10.0);
        assert_eq!(m.left(), 10.0);
        assert_eq!(MarginInfo::new(), MarginInfo::with_margins(0.0, 0.0, 0.0, 0.0));
    }
}
