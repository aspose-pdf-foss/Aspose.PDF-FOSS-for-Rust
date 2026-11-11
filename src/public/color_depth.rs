//! `Aspose.Pdf.Devices.ColorDepth` — output colour-depth selector
//! consumed by [`TiffSettings`](crate::TiffSettings).
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Devices.ColorDepth` enum: `Default=0`,
//! `Format24bpp=1`, `Format8bpp=2`, `Format4bpp=3`,
//! `Format1bpp=4`.

/// Pixel-depth selector for TIFF output.
///
/// `Default` defers to the canonical engine default (24 bpp RGB
/// on the v1 Rust track); the explicit `FormatNbpp` variants
/// request the matching packed-pixel layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ColorDepth {
    /// Engine default — canonical Aspose.PDF picks the
    /// implementation-default depth (24 bpp RGB on the v1 Rust
    /// track).
    #[default]
    Default = 0,
    /// 24 bpp RGB.
    Format24bpp = 1,
    /// 8 bpp grayscale or palette.
    Format8bpp = 2,
    /// 4 bpp palette.
    Format4bpp = 3,
    /// 1 bpp bilevel (black/white).
    Format1bpp = 4,
}
