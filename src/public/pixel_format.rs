//! `Aspose.Pdf.BitmapInfo.PixelFormat` — the packed-pixel layout of a
//! [`BitmapInfo`](crate::BitmapInfo) buffer, per
//! `capabilities/pixel_format.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum so the
//! discriminant round-trips through interop.

/// Packed-pixel layout of a [`BitmapInfo`](crate::BitmapInfo) buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PixelFormat {
    /// 24 bpp, three bytes per pixel in `R, G, B` order.
    Rgb24 = 0,
    /// 24 bpp, three bytes per pixel in `B, G, R` order (Windows
    /// DIB native).
    Bgr24 = 1,
    /// 32 bpp, four bytes per pixel in `R, G, B, A` order.
    Rgba32 = 2,
    /// 32 bpp, four bytes per pixel in `A, R, G, B` order
    /// (.NET `System.Drawing` native).
    Argb32 = 3,
    /// 32 bpp, four bytes per pixel in `B, G, R, A` order
    /// (Windows DIB with alpha).
    Bgra32 = 4,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_match_canonical() {
        assert_eq!(PixelFormat::Rgb24 as i32, 0);
        assert_eq!(PixelFormat::Bgr24 as i32, 1);
        assert_eq!(PixelFormat::Rgba32 as i32, 2);
        assert_eq!(PixelFormat::Argb32 as i32, 3);
        assert_eq!(PixelFormat::Bgra32 as i32, 4);
    }
}
