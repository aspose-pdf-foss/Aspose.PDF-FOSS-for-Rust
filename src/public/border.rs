//! `Aspose.Pdf.Annotations.Border` — the border style of an
//! annotation, per `capabilities/border.yaml`.
//!
//! The canonical constructor takes the owning annotation (used for
//! appearance regeneration); the Rust border is a standalone value
//! carrier, so the annotation argument is accepted for parity but
//! not retained.

use crate::public::annotation::Annotation;
use crate::public::border_effect::BorderEffect;
use crate::public::border_style::BorderStyle;
use crate::public::dash::Dash;

/// An annotation border (the `/BS` / `/Border` entry).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Border {
    width: i32,
    effect_intensity: i32,
    h_corner_radius: f64,
    v_corner_radius: f64,
    style: BorderStyle,
    effect: BorderEffect,
    dash: Dash,
}

impl Border {
    /// Construct a default border for the given annotation. The
    /// annotation is accepted for canonical parity but not retained.
    pub fn new(_annotation: &dyn Annotation) -> Self {
        Self::default()
    }

    /// The border width, in points.
    pub fn width(&self) -> i32 {
        self.width
    }
    /// Set the border width.
    pub fn set_width(&mut self, value: i32) {
        self.width = value;
    }

    /// The cloud-effect intensity (`0`–`2`), written as `/BE /I`
    /// beside the [`effect`](Border::effect) that asks for it.
    pub fn effect_intensity(&self) -> i32 {
        self.effect_intensity
    }
    /// Set the cloud-effect intensity.
    pub fn set_effect_intensity(&mut self, value: i32) {
        self.effect_intensity = value;
    }

    /// The horizontal corner radius.
    /// **Carried, not written** — and not for want of a place in the
    /// format, but because the only place is one that would be ignored.
    /// A corner radius lives in the older `/Border` array (§12.5.2,
    /// `[h v w]`) and has no counterpart in the `/BS` dictionary, and
    /// §12.5.2 makes `/BS` supersede `/Border` wherever both appear.
    /// This writer emits `/BS` for any border with a width, so a
    /// `/Border` array beside it would be inert — and writing an entry
    /// that provably takes no effect says something untrue about the
    /// document.
    pub fn h_corner_radius(&self) -> f64 {
        self.h_corner_radius
    }
    /// Set the horizontal corner radius.
    pub fn set_h_corner_radius(&mut self, value: f64) {
        self.h_corner_radius = value;
    }

    /// The vertical corner radius.
    /// **Carried, not written** — see
    /// [`h_corner_radius`](Border::h_corner_radius).
    pub fn v_corner_radius(&self) -> f64 {
        self.v_corner_radius
    }
    /// Set the vertical corner radius.
    pub fn set_v_corner_radius(&mut self, value: f64) {
        self.v_corner_radius = value;
    }

    /// The border line style.
    pub fn style(&self) -> BorderStyle {
        self.style
    }
    /// Set the border line style.
    pub fn set_style(&mut self, value: BorderStyle) {
        self.style = value;
    }

    /// The border effect (none / cloudy).
    /// The border effect. [`Cloudy`](crate::BorderEffect::Cloudy) is
    /// written as `/BE /S /C` (§12.5.4 Table 167) with the intensity
    /// beside it; `None` is what an absent `/BE` already means.
    pub fn effect(&self) -> BorderEffect {
        self.effect
    }
    /// Set the border effect.
    pub fn set_effect(&mut self, value: BorderEffect) {
        self.effect = value;
    }

    /// The border's dash pattern.
    pub fn dash(&self) -> &Dash {
        &self.dash
    }
    /// Set the border's dash pattern.
    pub fn set_dash(&mut self, value: Dash) {
        self.dash = value;
    }
}
