//! `Aspose.Pdf.IIndexBitmapConverter` — pluggable palette-
//! quantisation strategy turning a packed RGBA [`BitmapInfo`]
//! into a palette-indexed bitmap for indexed-colour TIFF output.
//!
//! Consumed by the converter-taking [`TiffDevice`](crate::TiffDevice)
//! constructors. The canonical .NET surface declares an
//! `interface` (not a class); Rust mirrors that with a trait,
//! and the `I` prefix is preserved per the identity-passthrough
//! translation rule (every `Aspose.Pdf.*` FQN maps to the
//! literal final segment so callers reach the type by the same
//! name they'd see in the .NET, C++, and TypeScript peer libs).
//!
//! The canonical .NET signatures take/return
//! `System.Drawing.Bitmap`; the Rust track substitutes
//! [`BitmapInfo`] per the
//! `targets/rust/translations.yaml` rule
//! `System.Drawing.Bitmap → Aspose.Pdf.BitmapInfo`.
//! Implementations receive an Rgba32-format [`BitmapInfo`] and
//! return a [`BitmapInfo`] whose pixel buffer carries at most
//! 2 / 16 / 256 distinct RGB triples for the 1 / 4 / 8 bpp
//! methods respectively. [`TiffDevice`](crate::TiffDevice)
//! reconstructs the palette + per-pixel index buffer from the
//! returned bytes when it emits indexed-mode TIFF output.
//!
//! On the v1 Rust track [`TiffDevice`](crate::TiffDevice) does
//! not yet emit indexed-mode strips (the `ColorDepth::FormatNbpp`
//! axes ride on [`TiffSettings`](crate::TiffSettings) but are
//! decoupled from the encoder until the bilevel-codec beat
//! lands). A converter passed via the ctors is stored on the
//! device for future use; it does not run during the v1
//! 24-bpp-RGB encode path.

use super::bitmap_info::BitmapInfo;

/// Pluggable palette-quantisation strategy for indexed-colour
/// TIFF output.
///
/// Implementations must accept any width/height and produce a
/// [`BitmapInfo`] of the same geometry whose pixel buffer
/// carries at most 2 / 16 / 256 distinct RGB triples for the
/// 1 / 4 / 8 bpp methods respectively. The output [`BitmapInfo`]
/// can use any [`PixelFormat`](crate::PixelFormat) — TiffDevice
/// extracts the palette by scanning unique colours, not by
/// inspecting the format.
#[allow(clippy::module_name_repetitions)]
pub trait IIndexBitmapConverter {
    /// Quantise `src` to ≤ 2 distinct RGB colours (1 bpp target).
    fn get_1bpp_image(&self, src: &BitmapInfo) -> BitmapInfo;

    /// Quantise `src` to ≤ 16 distinct RGB colours (4 bpp,
    /// 16-colour palette).
    fn get_4bpp_image(&self, src: &BitmapInfo) -> BitmapInfo;

    /// Quantise `src` to ≤ 256 distinct RGB colours (8 bpp,
    /// 256-colour palette).
    fn get_8bpp_image(&self, src: &BitmapInfo) -> BitmapInfo;
}
