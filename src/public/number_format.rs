//! `Aspose.Pdf.Annotations.Measure.NumberFormat` — one number-format
//! entry describing how a measurement axis is labelled and scaled,
//! per `capabilities/number_format.yaml`.
//!
//! The `Measure`-taking constructor is dropped (see
//! `targets/rust/drops.yaml`); construct via `Default` and the
//! setters. `convresion_factor` mirrors the canonical (misspelled)
//! `ConvresionFactor` member name.
//!
//! Written as one §12.9 Table 262 dictionary inside the `/Measure` of
//! the annotation that carries it (see [`Measure`](crate::Measure)).
//!
//! Two notes on the mapping. Table 262 gives `/U` and `/C` as required,
//! so a format with no unit label or a zero conversion factor is a
//! placeholder and is left out rather than written as an invalid
//! dictionary. And `precision` and `denominator` share one key: `/D` is
//! "a positive integer denominator, constrained to be a power of 10
//! when the value of F is D", so a decimal format's `/D` is its
//! precision and a fractional format's is its denominator, while
//! rounding and truncating have no fractional part for either to
//! describe.

use crate::public::fraction_style::FractionStyle;

/// One number-format entry of a measurement's format list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberFormat {
    unit_label: String,
    convresion_factor: f64,
    precision: i32,
    denominator: i32,
    force_denominator: bool,
    thousands_separator: String,
    fraction_separator: String,
    before_text: String,
    after_text: String,
    fraction_displayment: FractionStyle,
}

impl NumberFormat {
    /// The unit label, e.g. `"mi"` (canonical `UnitLabel`).
    pub fn unit_label(&self) -> &str {
        &self.unit_label
    }
    /// Set the unit label.
    pub fn set_unit_label(&mut self, value: String) {
        self.unit_label = value;
    }

    /// The factor converting default user-space units to this format's
    /// units (canonical `ConvresionFactor` — spelling mirrored).
    pub fn convresion_factor(&self) -> f64 {
        self.convresion_factor
    }
    /// Set the conversion factor.
    pub fn set_convresion_factor(&mut self, value: f64) {
        self.convresion_factor = value;
    }

    /// The precision of the displayed value (canonical `Precision`).
    pub fn precision(&self) -> i32 {
        self.precision
    }
    /// Set the precision.
    pub fn set_precision(&mut self, value: i32) {
        self.precision = value;
    }

    /// The denominator for fractional display (canonical
    /// `Denominator`).
    pub fn denominator(&self) -> i32 {
        self.denominator
    }
    /// Set the denominator.
    pub fn set_denominator(&mut self, value: i32) {
        self.denominator = value;
    }

    /// Whether the denominator is forced (canonical
    /// `ForceDenominator`).
    pub fn force_denominator(&self) -> bool {
        self.force_denominator
    }
    /// Set whether the denominator is forced.
    pub fn set_force_denominator(&mut self, value: bool) {
        self.force_denominator = value;
    }

    /// The thousands separator (canonical `ThousandsSeparator`).
    pub fn thousands_separator(&self) -> &str {
        &self.thousands_separator
    }
    /// Set the thousands separator.
    pub fn set_thousands_separator(&mut self, value: String) {
        self.thousands_separator = value;
    }

    /// The fraction separator (canonical `FractionSeparator`).
    pub fn fraction_separator(&self) -> &str {
        &self.fraction_separator
    }
    /// Set the fraction separator.
    pub fn set_fraction_separator(&mut self, value: String) {
        self.fraction_separator = value;
    }

    /// Text prepended to the formatted value (canonical `BeforeText`).
    pub fn before_text(&self) -> &str {
        &self.before_text
    }
    /// Set the before-text.
    pub fn set_before_text(&mut self, value: String) {
        self.before_text = value;
    }

    /// Text appended to the formatted value (canonical `AfterText`).
    pub fn after_text(&self) -> &str {
        &self.after_text
    }
    /// Set the after-text.
    pub fn set_after_text(&mut self, value: String) {
        self.after_text = value;
    }

    /// How fractional values are displayed (canonical
    /// `FractionDisplayment`).
    pub fn fraction_displayment(&self) -> FractionStyle {
        self.fraction_displayment
    }
    /// Set the fraction display style.
    pub fn set_fraction_displayment(&mut self, value: FractionStyle) {
        self.fraction_displayment = value;
    }
}
