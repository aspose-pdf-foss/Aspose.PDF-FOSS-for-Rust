//! `Aspose.Pdf.BorderInfo` — a per-edge table / box border, per
//! `capabilities/border_info.yaml`.
//!
//! Carries a [`GraphInfo`] styling per edge (left / right / top /
//! bottom) plus a corner radius. The constructors configure the
//! edges named by a [`BorderSide`] flag set; unnamed edges stay at
//! zero width (no border). The `ICloneable.Clone` member is dropped
//! in favour of the derived `Clone`.

use crate::public::border_side::BorderSide;
use crate::public::color::Color;
use crate::public::graph_info::GraphInfo;

/// A four-edge border.
#[derive(Debug, Clone, PartialEq)]
pub struct BorderInfo {
    left: GraphInfo,
    right: GraphInfo,
    top: GraphInfo,
    bottom: GraphInfo,
    rounded_border_radius: f64,
}

/// A zero-width edge (no border drawn).
fn zero_edge() -> GraphInfo {
    let mut g = GraphInfo::new();
    g.set_line_width(0.0);
    g
}

impl Default for BorderInfo {
    fn default() -> Self {
        Self {
            left: zero_edge(),
            right: zero_edge(),
            top: zero_edge(),
            bottom: zero_edge(),
            rounded_border_radius: 0.0,
        }
    }
}

impl BorderInfo {
    /// Construct a border with every edge at zero width.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the named edges with a default 1pt styling.
    pub fn with_side(side: BorderSide) -> Self {
        let mut b = Self::new();
        b.configure(side, |g| *g = GraphInfo::new());
        b
    }

    /// Configure the named edges with `border_width` (points).
    pub fn with_side_width(side: BorderSide, border_width: f32) -> Self {
        let mut b = Self::new();
        b.configure(side, |g| g.set_line_width(border_width));
        b
    }

    /// Configure the named edges with width and colour.
    pub fn with_side_width_color(
        side: BorderSide,
        border_width: f32,
        border_color: Color,
    ) -> Self {
        let mut b = Self::new();
        b.configure(side, |g| {
            g.set_line_width(border_width);
            g.set_color(border_color);
        });
        b
    }

    /// Configure the named edges with a default width and a colour.
    pub fn with_side_color(side: BorderSide, border_color: Color) -> Self {
        let mut b = Self::new();
        b.configure(side, |g| {
            g.set_line_width(1.0);
            g.set_color(border_color);
        });
        b
    }

    /// Configure the named edges with a full [`GraphInfo`] styling.
    pub fn with_side_graph_info(side: BorderSide, info: GraphInfo) -> Self {
        let mut b = Self::new();
        b.configure(side, |g| *g = info.clone());
        b
    }

    fn configure(&mut self, side: BorderSide, f: impl Fn(&mut GraphInfo)) {
        if side.contains(BorderSide::LEFT) {
            f(&mut self.left);
        }
        if side.contains(BorderSide::RIGHT) {
            f(&mut self.right);
        }
        if side.contains(BorderSide::TOP) {
            f(&mut self.top);
        }
        if side.contains(BorderSide::BOTTOM) {
            f(&mut self.bottom);
        }
    }

    /// The left-edge styling.
    pub fn left(&self) -> &GraphInfo {
        &self.left
    }
    /// Set the left-edge styling.
    pub fn set_left(&mut self, value: GraphInfo) {
        self.left = value;
    }

    /// The right-edge styling.
    pub fn right(&self) -> &GraphInfo {
        &self.right
    }
    /// Set the right-edge styling.
    pub fn set_right(&mut self, value: GraphInfo) {
        self.right = value;
    }

    /// The top-edge styling.
    pub fn top(&self) -> &GraphInfo {
        &self.top
    }
    /// Set the top-edge styling.
    pub fn set_top(&mut self, value: GraphInfo) {
        self.top = value;
    }

    /// The bottom-edge styling.
    pub fn bottom(&self) -> &GraphInfo {
        &self.bottom
    }
    /// Set the bottom-edge styling.
    pub fn set_bottom(&mut self, value: GraphInfo) {
        self.bottom = value;
    }

    /// The corner radius, in points.
    pub fn rounded_border_radius(&self) -> f64 {
        self.rounded_border_radius
    }
    /// Set the corner radius, in points.
    pub fn set_rounded_border_radius(&mut self, value: f64) {
        self.rounded_border_radius = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_no_edges() {
        let b = BorderInfo::new();
        assert_eq!(b.left().line_width(), 0.0);
        assert_eq!(b.top().line_width(), 0.0);
        assert_eq!(b.rounded_border_radius(), 0.0);
    }

    #[test]
    fn constructors_configure_named_edges() {
        let all = BorderInfo::with_side_width_color(
            BorderSide::ALL,
            2.0,
            Color::from_rgb(0.0, 0.0, 1.0),
        );
        assert_eq!(all.left().line_width(), 2.0);
        assert_eq!(all.bottom().line_width(), 2.0);
        assert_eq!(all.right().color().a(), 1.0);

        let lr = BorderInfo::with_side_width(
            BorderSide::LEFT | BorderSide::RIGHT,
            3.0,
        );
        assert_eq!(lr.left().line_width(), 3.0);
        assert_eq!(lr.right().line_width(), 3.0);
        assert_eq!(lr.top().line_width(), 0.0);
        assert_eq!(lr.bottom().line_width(), 0.0);

        let mut g = GraphInfo::new();
        g.set_line_width(5.0);
        let mut top = BorderInfo::with_side_graph_info(BorderSide::TOP, g);
        assert_eq!(top.top().line_width(), 5.0);
        assert_eq!(top.left().line_width(), 0.0);
        top.set_rounded_border_radius(4.0);
        assert_eq!(top.rounded_border_radius(), 4.0);
    }
}
