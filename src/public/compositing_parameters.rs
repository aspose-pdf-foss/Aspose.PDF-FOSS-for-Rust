//! `Aspose.Pdf.CompositingParameters` — the blend / filter / mask state
//! under which an image is composited, per
//! `capabilities/compositing_parameters.yaml`.
//!
//! A read-only carrier of the [`blend_mode`](CompositingParameters::blend_mode)
//! (from the graphics-state `/BM`), the [`filter_type`](CompositingParameters::filter_type)
//! (the image's compression codec), and whether the image
//! [`is_masked`](CompositingParameters::is_masked). Reached via
//! [`ImagePlacement::compositing_parameters`](crate::ImagePlacement::compositing_parameters).

use crate::public::blend_mode::BlendMode;
use crate::public::image_filter_type::ImageFilterType;

/// The compositing state applied when an image is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositingParameters {
    blend_mode: BlendMode,
    filter_type: ImageFilterType,
    is_masked: bool,
}

impl CompositingParameters {
    /// Construct from a blend mode, defaulting the filter to `Jpeg` and the
    /// masked flag to `false`.
    pub fn new(blend_mode: BlendMode) -> Self {
        Self {
            blend_mode,
            filter_type: ImageFilterType::Jpeg,
            is_masked: false,
        }
    }

    /// Construct from a blend mode and filter type, defaulting the masked
    /// flag to `false`.
    pub fn with_filter(blend_mode: BlendMode, filter_type: ImageFilterType) -> Self {
        Self {
            filter_type,
            ..Self::new(blend_mode)
        }
    }

    /// Construct from a blend mode, filter type and masked flag.
    pub fn with_mask(blend_mode: BlendMode, filter_type: ImageFilterType, is_masked: bool) -> Self {
        Self {
            is_masked,
            ..Self::with_filter(blend_mode, filter_type)
        }
    }

    /// The blend mode in effect where the image is drawn.
    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    /// The image's compression filter.
    pub fn filter_type(&self) -> ImageFilterType {
        self.filter_type
    }

    /// Whether the image carries a mask (a soft `/SMask` or a `/Mask`).
    pub fn is_masked(&self) -> bool {
        self.is_masked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctor_defaults() {
        let c = CompositingParameters::new(BlendMode::Normal);
        assert_eq!(c.blend_mode(), BlendMode::Normal);
        assert_eq!(c.filter_type(), ImageFilterType::Jpeg);
        assert!(!c.is_masked());
    }

    #[test]
    fn ctor_progressive_overrides() {
        let c = CompositingParameters::with_filter(BlendMode::Multiply, ImageFilterType::Flate);
        assert_eq!(c.blend_mode(), BlendMode::Multiply);
        assert_eq!(c.filter_type(), ImageFilterType::Flate);
        assert!(!c.is_masked());

        let c = CompositingParameters::with_mask(BlendMode::Screen, ImageFilterType::CCITTFax, true);
        assert_eq!(c.blend_mode(), BlendMode::Screen);
        assert_eq!(c.filter_type(), ImageFilterType::CCITTFax);
        assert!(c.is_masked());
    }
}
