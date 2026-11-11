//! `Aspose.Pdf.Devices.CompressionType` — TIFF compression-scheme
//! selector consumed by [`TiffSettings`](crate::TiffSettings).
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Devices.CompressionType` enum: `LZW=0`,
//! `CCITT4=1`, `CCITT3=2`, `RLE=3`, `None=4`. Note that `None`
//! is *not* the zero value — the canonical default is LZW.

/// TIFF compression-scheme selector.
///
/// The v1 Rust [`TiffDevice`](crate::TiffDevice) routes `LZW` →
/// `internal::tiff_encoder::Compression::Lzw`, `RLE` →
/// `PackBits`, and `None` → `None`. `CCITT3` / `CCITT4` are
/// only legal alongside [`ColorDepth::Format1bpp`](crate::ColorDepth)
/// — when paired with any other depth the device falls back to
/// uncompressed strips (the canonical .NET surface raises;
/// Rust degrades gracefully to keep RGB/palette workflows
/// usable when callers forget to set `Depth = Format1bpp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum CompressionType {
    /// Lempel–Ziv–Welch — the canonical default.
    #[default]
    Lzw = 0,
    /// CCITT Group 4 facsimile (bilevel only). Requires
    /// `Depth = Format1bpp`; otherwise the device falls back
    /// to uncompressed strips.
    Ccitt4 = 1,
    /// CCITT Group 3 1D facsimile (bilevel only). Requires
    /// `Depth = Format1bpp`; otherwise the device falls back
    /// to uncompressed strips.
    Ccitt3 = 2,
    /// PackBits run-length encoding.
    Rle = 3,
    /// Uncompressed.
    None = 4,
}
