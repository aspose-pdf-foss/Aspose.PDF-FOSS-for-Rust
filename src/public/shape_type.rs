//! `Aspose.Pdf.Devices.ShapeType` — orientation hint consumed by
//! [`TiffSettings`](crate::TiffSettings).
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Devices.ShapeType` enum: `None=0`,
//! `Landscape=1`, `Portrait=2`.

/// Page-orientation hint for the rendering pipeline. `None`
/// means "honour the page's native orientation"; the
/// `Landscape` / `Portrait` variants request a rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ShapeType {
    /// Honour the page's native orientation.
    #[default]
    None = 0,
    /// Force landscape output.
    Landscape = 1,
    /// Force portrait output.
    Portrait = 2,
}
