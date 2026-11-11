//! `Aspose.Pdf.BitmapInfo` — flat raster image carrier.
//!
//! Pure POD value type per `capabilities/bitmap_info.yaml`:
//! one ctor + four read-only accessors, paired with the
//! [`PixelFormat`] enum of five packed-pixel layouts. Substitutes
//! the canonical .NET surface's `System.Drawing.Bitmap` return type
//! on raster-producing devices (see the `System.Drawing.Bitmap →
//! Aspose.Pdf.BitmapInfo` substitution in
//! `targets/rust/translations.yaml`), keeping the public API free
//! of platform-specific image-codec dependencies.

use crate::public::pixel_format::PixelFormat;

/// Packed raster image carrier.
///
/// [`pixel_bytes`](BitmapInfo::pixel_bytes) is a row-major byte
/// buffer holding [`width`](BitmapInfo::width) ×
/// [`height`](BitmapInfo::height) pixels in the
/// [`format`](BitmapInfo::format) packed-pixel layout. The carrier
/// neither validates nor reshapes the buffer — callers and devices
/// agree on the geometry by convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitmapInfo {
    pixel_bytes: Vec<u8>,
    width: i32,
    height: i32,
    format: PixelFormat,
}

impl BitmapInfo {
    /// Construct a [`BitmapInfo`] from a row-major packed-pixel
    /// buffer with the declared `width`, `height`, and pixel
    /// `format`.
    pub fn new(
        pixel_bytes: Vec<u8>,
        width: i32,
        height: i32,
        format: PixelFormat,
    ) -> Self {
        Self { pixel_bytes, width, height, format }
    }

    /// Packed-pixel byte buffer in [`format`](BitmapInfo::format)
    /// layout.
    pub fn pixel_bytes(&self) -> &[u8] {
        &self.pixel_bytes
    }

    /// Image width in pixels.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Image height in pixels.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Packed-pixel layout of [`pixel_bytes`](BitmapInfo::pixel_bytes).
    pub fn format(&self) -> PixelFormat {
        self.format
    }
}
