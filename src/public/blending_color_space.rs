//! `Aspose.Pdf.Facades.BlendingColorSpace` — transparency blending colour
//! space, per `capabilities/blending_color_space.yaml`. Ordinals match
//! canonical 26.4.0.

/// A transparency blending colour space.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum BlendingColorSpace {
    /// Leave the blending space unchanged.
    #[default]
    DontChange = 0,
    /// Pick automatically.
    Auto = 1,
    /// Device RGB.
    DeviceRGB = 2,
    /// Device CMYK.
    DeviceCMYK = 3,
}
