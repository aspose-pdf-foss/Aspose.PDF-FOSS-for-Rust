//! `Aspose.Pdf.BlendMode` — the separable / non-separable blend mode
//! used when compositing content, per `capabilities/blend_mode.yaml`.
//!
//! Mirrors the PDF `/BM` graphics-state values. Variant ordinals match the
//! canonical Aspose.PDF 26.4.0 enum.

/// The blend mode used when compositing content onto a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum BlendMode {
    /// `Normal` — the default; the source replaces the backdrop.
    #[default]
    Normal = 0,
    /// `Multiply`.
    Multiply = 1,
    /// `Screen`.
    Screen = 2,
    /// `Overlay`.
    Overlay = 3,
    /// `Darken`.
    Darken = 4,
    /// `Lighten`.
    Lighten = 5,
    /// `ColorDodge`.
    ColorDodge = 6,
    /// `ColorBurn`.
    ColorBurn = 7,
    /// `HardLight`.
    HardLight = 8,
    /// `SoftLight`.
    SoftLight = 9,
    /// `Difference`.
    Difference = 10,
    /// `Exclusion`.
    Exclusion = 11,
    /// `Hue`.
    Hue = 12,
    /// `Saturation`.
    Saturation = 13,
    /// `Color`.
    Color = 14,
    /// `Luminosity`.
    Luminosity = 15,
    /// `Compatible` — the legacy alias for `Normal`.
    Compatible = 16,
}
