//! `Aspose.Pdf.ColorType` — the colour classification of an image's
//! samples, per `capabilities/color_type.yaml`.
//!
//! Reported by [`XImage::get_color_type`](crate::XImage::get_color_type),
//! derived from the image's `/ColorSpace`, `/BitsPerComponent` and
//! `/ImageMask` (or, for an in-memory image, its encoded header).
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The colour classification of an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ColorType {
    /// A colour image (RGB / CMYK / indexed / ICC-based).
    #[default]
    Rgb = 0,
    /// A grayscale image (more than one bit per component).
    Grayscale = 1,
    /// A bilevel (one-bit) black-and-white image.
    BlackAndWhite = 2,
    /// The colour type could not be determined.
    Undefined = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_and_default() {
        assert_eq!(ColorType::default(), ColorType::Rgb);
        assert_eq!(ColorType::Rgb as i32, 0);
        assert_eq!(ColorType::Grayscale as i32, 1);
        assert_eq!(ColorType::BlackAndWhite as i32, 2);
        assert_eq!(ColorType::Undefined as i32, 3);
    }
}
