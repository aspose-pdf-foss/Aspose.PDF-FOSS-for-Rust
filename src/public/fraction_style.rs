//! `Aspose.Pdf.Annotations.Measure.NumberFormat.FractionStyle` — how
//! fractional measurement values are displayed, per
//! `capabilities/fraction_style.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// How fractional measurement values are displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FractionStyle {
    /// Show fractional values as a decimal fraction.
    #[default]
    ShowAsDecimal = 0,
    /// Show the fractional value as a fraction.
    ShowAsFraction = 1,
    /// Round fractional values to the nearest whole integer.
    Round = 2,
    /// Truncate to whole units.
    Truncate = 3,
}
