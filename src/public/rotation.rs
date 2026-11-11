//! `Aspose.Pdf.Rotation` — page-rotation selector, per
//! `capabilities/rotation.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Rotation` enum. The reference ships the
//! `None` / `on90` / `on180` / `on270` quadrant subset; the
//! canonical `on360` value (a full turn, equivalent to `None`)
//! is dropped to match the C++ reference selection.

/// Page rotation in 90° increments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum Rotation {
    /// No rotation.
    #[default]
    None = 0,
    /// Rotate 90° clockwise.
    On90 = 1,
    /// Rotate 180°.
    On180 = 2,
    /// Rotate 270° clockwise (90° counter-clockwise).
    On270 = 3,
}
