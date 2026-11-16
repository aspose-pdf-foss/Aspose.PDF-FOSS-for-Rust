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
    /// Multiplies backdrop and source, so the result is never lighter than either.
    Multiply = 1,
    /// Multiplies the complements, so the result is never darker than either.
    Screen = 2,
    /// Multiplies or screens according to the backdrop, keeping its highlights and shadows.
    Overlay = 3,
    /// Keeps whichever of backdrop and source is the darker.
    Darken = 4,
    /// Keeps whichever of backdrop and source is the lighter.
    Lighten = 5,
    /// Brightens the backdrop to reflect the source.
    ColorDodge = 6,
    /// Darkens the backdrop to reflect the source.
    ColorBurn = 7,
    /// Multiplies or screens according to the source, as under a harsh spotlight.
    HardLight = 8,
    /// Darkens or lightens according to the source, as under a diffuse spotlight.
    SoftLight = 9,
    /// Subtracts the darker of the two colours from the lighter.
    Difference = 10,
    /// Like `Difference`, but with lower contrast.
    Exclusion = 11,
    /// The hue of the source with the saturation and luminosity of the backdrop.
    Hue = 12,
    /// The saturation of the source with the hue and luminosity of the backdrop.
    Saturation = 13,
    /// The hue and saturation of the source with the luminosity of the backdrop.
    Color = 14,
    /// The luminosity of the source with the hue and saturation of the backdrop.
    Luminosity = 15,
    /// `Compatible` — the legacy alias for `Normal`.
    Compatible = 16,
}
