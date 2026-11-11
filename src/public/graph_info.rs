//! `Aspose.Pdf.GraphInfo` — stroke / fill styling for a graph or
//! table border, per `capabilities/graph_info.yaml`.
//!
//! Carries the line width, the stroke and fill colours and the dash
//! pattern — all of which reach the page — together with the affine
//! descriptors (skew, scale, rotation), the doubled-line flag and the
//! shape's coordinates, which do NOT. See [`GraphInfo::is_doubled`]
//! for what that means and why. The `ICloneable.Clone` member is
//! dropped in favour of the derived `Clone`.

use crate::public::color::Color;

/// Stroke / fill styling.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphInfo {
    x: f64,
    y: f64,
    line_width: f32,
    color: Color,
    dash_array: Vec<i32>,
    dash_phase: i32,
    fill_color: Color,
    is_doubled: bool,
    skew_angle_x: f64,
    skew_angle_y: f64,
    scaling_rate_x: f64,
    scaling_rate_y: f64,
    rotation_angle: f64,
}

impl Default for GraphInfo {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            line_width: 1.0,
            color: Color::empty(),
            dash_array: Vec::new(),
            dash_phase: 0,
            fill_color: Color::empty(),
            is_doubled: false,
            skew_angle_x: 0.0,
            skew_angle_y: 0.0,
            scaling_rate_x: 1.0,
            scaling_rate_y: 1.0,
            rotation_angle: 0.0,
        }
    }
}

impl GraphInfo {
    /// Construct default styling (1pt line, unit scale).
    pub fn new() -> Self {
        Self::default()
    }

    /// The x coordinate of the styled shape (read-only).
    ///
    /// Always zero: this surface has no setter for it and nothing
    /// populates it — a shape carries its own geometry, and a
    /// [`Graph`](crate::Graph) is positioned by its `left` / `top`.
    pub fn x(&self) -> f64 {
        self.x
    }
    /// The y coordinate of the styled shape (read-only). Always zero,
    /// for the reason given on [`GraphInfo::x`].
    pub fn y(&self) -> f64 {
        self.y
    }

    /// The stroke line width, in points.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }
    /// Set the stroke line width, in points.
    pub fn set_line_width(&mut self, value: f32) {
        self.line_width = value;
    }

    /// The stroke colour.
    pub fn color(&self) -> Color {
        self.color
    }
    /// Set the stroke colour.
    pub fn set_color(&mut self, value: Color) {
        self.color = value;
    }

    /// The dash pattern (alternating on/off lengths).
    pub fn dash_array(&self) -> &[i32] {
        &self.dash_array
    }
    /// Set the dash pattern.
    pub fn set_dash_array(&mut self, value: Vec<i32>) {
        self.dash_array = value;
    }

    /// The dash phase (offset into the dash pattern).
    pub fn dash_phase(&self) -> i32 {
        self.dash_phase
    }
    /// Set the dash phase.
    pub fn set_dash_phase(&mut self, value: i32) {
        self.dash_phase = value;
    }

    /// The fill colour.
    pub fn fill_color(&self) -> Color {
        self.fill_color
    }
    /// Set the fill colour.
    pub fn set_fill_color(&mut self, value: Color) {
        self.fill_color = value;
    }

    /// Whether the line is drawn doubled.
    ///
    /// **Carried, not applied.** The value round-trips through this
    /// object and no further: nothing reads it when a shape is written
    /// into a page's content stream, so the stroke is single either
    /// way. The same holds for [`skew_angle_x`](GraphInfo::skew_angle_x),
    /// [`skew_angle_y`](GraphInfo::skew_angle_y),
    /// [`scaling_rate_x`](GraphInfo::scaling_rate_x),
    /// [`scaling_rate_y`](GraphInfo::scaling_rate_y) and
    /// [`rotation_angle`](GraphInfo::rotation_angle).
    ///
    /// It is stated rather than quietly true because the catalogue
    /// defines only the type of these members, not what they do to a
    /// shape, and the reference C++ track stores them without reading
    /// them either — so there is no behaviour to copy and none is
    /// invented here.
    pub fn is_doubled(&self) -> bool {
        self.is_doubled
    }
    /// Set whether the line is drawn doubled.
    pub fn set_is_doubled(&mut self, value: bool) {
        self.is_doubled = value;
    }

    /// The x skew angle, in degrees.
    /// **Carried, not applied** — see [`GraphInfo::is_doubled`].
    pub fn skew_angle_x(&self) -> f64 {
        self.skew_angle_x
    }
    /// Set the x skew angle, in degrees.
    pub fn set_skew_angle_x(&mut self, value: f64) {
        self.skew_angle_x = value;
    }

    /// The y skew angle, in degrees.
    /// **Carried, not applied** — see [`GraphInfo::is_doubled`].
    pub fn skew_angle_y(&self) -> f64 {
        self.skew_angle_y
    }
    /// Set the y skew angle, in degrees.
    pub fn set_skew_angle_y(&mut self, value: f64) {
        self.skew_angle_y = value;
    }

    /// The x scaling rate (1.0 = no scaling).
    /// **Carried, not applied** — see [`GraphInfo::is_doubled`].
    pub fn scaling_rate_x(&self) -> f64 {
        self.scaling_rate_x
    }
    /// Set the x scaling rate.
    pub fn set_scaling_rate_x(&mut self, value: f64) {
        self.scaling_rate_x = value;
    }

    /// The y scaling rate (1.0 = no scaling).
    /// **Carried, not applied** — see [`GraphInfo::is_doubled`].
    pub fn scaling_rate_y(&self) -> f64 {
        self.scaling_rate_y
    }
    /// Set the y scaling rate.
    pub fn set_scaling_rate_y(&mut self, value: f64) {
        self.scaling_rate_y = value;
    }

    /// The rotation angle, in degrees.
    /// **Carried, not applied** — see [`GraphInfo::is_doubled`].
    pub fn rotation_angle(&self) -> f64 {
        self.rotation_angle
    }
    /// Set the rotation angle, in degrees.
    pub fn set_rotation_angle(&mut self, value: f64) {
        self.rotation_angle = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_properties() {
        let mut g = GraphInfo::new();
        assert_eq!(g.line_width(), 1.0);
        assert_eq!(g.scaling_rate_x(), 1.0);
        assert!(!g.is_doubled());
        assert!(g.dash_array().is_empty());
        assert_eq!(g.x(), 0.0);

        g.set_line_width(2.5);
        g.set_color(Color::from_rgb(1.0, 0.0, 0.0));
        g.set_dash_array(vec![3, 2]);
        g.set_dash_phase(1);
        g.set_is_doubled(true);
        g.set_rotation_angle(45.0);

        assert_eq!(g.line_width(), 2.5);
        assert_eq!(g.color().a(), 1.0);
        assert_eq!(g.dash_array(), &[3, 2]);
        assert_eq!(g.dash_phase(), 1);
        assert!(g.is_doubled());
        assert_eq!(g.rotation_angle(), 45.0);
    }
}
