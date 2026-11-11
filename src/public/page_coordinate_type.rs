//! `Aspose.Pdf.PageCoordinateType` — page-box selector for the
//! rendering pipeline. Currently consumed by
//! [`TiffSettings`](crate::TiffSettings); a future beat will
//! also wire the [`ImageDevice`](crate::ImageDevice) accessor
//! (which still drops from the v1 surface until the rendering-
//! options cluster ships).
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.PageCoordinateType` enum: `MediaBox=0`,
//! `CropBox=1`.

/// Selects which page-level box (MediaBox vs CropBox) drives
/// the renderer's output canvas size. `CropBox` is the canonical
/// engine default — it produces the page's visible region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
#[derive(Default)]
pub enum PageCoordinateType {
    /// Use the page's MediaBox.
    MediaBox = 0,
    /// Use the page's CropBox (canonical default).
    #[default]
    CropBox = 1,
}

