//! `Aspose.Pdf.ColorSpace` — a device colour space, per
//! `capabilities/color_space.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// A device colour space.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ColorSpace {
    /// Device RGB colour space.
    #[default]
    DeviceRGB = 0,
    /// Device CMYK colour space.
    DeviceCMYK = 1,
    /// Device grayscale colour space.
    DeviceGray = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(ColorSpace::DeviceRGB as i32, 0);
        assert_eq!(ColorSpace::DeviceCMYK as i32, 1);
        assert_eq!(ColorSpace::DeviceGray as i32, 2);
    }

    #[test]
    fn default_is_device_rgb() {
        assert_eq!(ColorSpace::default(), ColorSpace::DeviceRGB);
    }
}
