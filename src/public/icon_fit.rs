//! `Aspose.Pdf.Forms.IconFit` — button-icon scaling options, per
//! `capabilities/icon_fit.yaml`.
//!
//! **Carried, not written.** `/IF` is a declaration inside a button's
//! `/MK` dictionary (§12.5.6.19 Table 190), not part of an appearance
//! stream — so there *is* a place for it. What is missing is the thing
//! it steers: the icon itself is `/MK /I`, an image XObject, a second
//! object, and the annotation writer emits one dictionary per
//! annotation. An icon fit with no icon would fit nothing, so it is
//! left in memory until a button can carry an icon.


use crate::public::scaling_mode::ScalingMode;
use crate::public::scaling_reason::ScalingReason;

/// Controls how a button icon is scaled to fit its field rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconFit {
    scaling_reason: ScalingReason,
    scaling_mode: ScalingMode,
    leftover_left: f64,
    leftover_bottom: f64,
    spread_on_border: bool,
}

impl Default for IconFit {
    fn default() -> Self {
        Self {
            scaling_reason: ScalingReason::Always,
            scaling_mode: ScalingMode::Proportional,
            leftover_left: 0.0,
            leftover_bottom: 0.0,
            spread_on_border: false,
        }
    }
}

impl IconFit {
    /// Construct default icon-fit options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Map a PDF scaling-reason name (`A` / `B` / `S` / `N`) to a
    /// [`ScalingReason`].
    pub fn name_to_scaling_reason(name: &str) -> ScalingReason {
        match name {
            "B" => ScalingReason::IconIsBigger,
            "S" => ScalingReason::IconIsSmaller,
            "N" => ScalingReason::Never,
            _ => ScalingReason::Always,
        }
    }
    /// Map a [`ScalingReason`] to its PDF name.
    pub fn scaling_reason_to_name(reason: ScalingReason) -> String {
        match reason {
            ScalingReason::Always => "A",
            ScalingReason::IconIsBigger => "B",
            ScalingReason::IconIsSmaller => "S",
            ScalingReason::Never => "N",
        }
        .to_string()
    }
    /// Map a PDF scaling-mode name (`A` / `P`) to a [`ScalingMode`].
    pub fn name_to_scaling_mode(name: &str) -> ScalingMode {
        match name {
            "A" => ScalingMode::Anamorphic,
            _ => ScalingMode::Proportional,
        }
    }
    /// Map a [`ScalingMode`] to its PDF name.
    pub fn scaling_mode_to_name(mode: ScalingMode) -> String {
        match mode {
            ScalingMode::Proportional => "P",
            ScalingMode::Anamorphic => "A",
        }
        .to_string()
    }

    /// The reason scaling is applied.
    pub fn scaling_reason(&self) -> ScalingReason { self.scaling_reason }
    /// Set the reason scaling is applied.
    pub fn set_scaling_reason(&mut self, v: ScalingReason) { self.scaling_reason = v; }
    /// The scaling mode (proportional / anamorphic).
    pub fn scaling_mode(&self) -> ScalingMode { self.scaling_mode }
    /// Set the scaling mode.
    pub fn set_scaling_mode(&mut self, v: ScalingMode) { self.scaling_mode = v; }
    /// The horizontal leftover-space fraction.
    pub fn leftover_left(&self) -> f64 { self.leftover_left }
    /// Set the horizontal leftover-space fraction.
    pub fn set_leftover_left(&mut self, v: f64) { self.leftover_left = v; }
    /// The vertical leftover-space fraction.
    pub fn leftover_bottom(&self) -> f64 { self.leftover_bottom }
    /// Set the vertical leftover-space fraction.
    pub fn set_leftover_bottom(&mut self, v: f64) { self.leftover_bottom = v; }
    /// Whether the icon spreads onto the border.
    pub fn spread_on_border(&self) -> bool { self.spread_on_border }
    /// Set whether the icon spreads onto the border.
    pub fn set_spread_on_border(&mut self, v: bool) { self.spread_on_border = v; }
}
