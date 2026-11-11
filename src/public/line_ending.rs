//! `Aspose.Pdf.Annotations.LineEnding` — annotation enum, per `capabilities/line_ending.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `LineEnding` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum LineEnding {
    /// `None`.
    #[default]
    None = 0,
    /// `Square`.
    Square = 1,
    /// `Circle`.
    Circle = 2,
    /// `Diamond`.
    Diamond = 3,
    /// `OpenArrow`.
    OpenArrow = 4,
    /// `ClosedArrow`.
    ClosedArrow = 5,
    /// `Butt`.
    Butt = 6,
    /// `ROpenArrow`.
    ROpenArrow = 7,
    /// `RClosedArrow`.
    RClosedArrow = 8,
    /// `Slash`.
    Slash = 9,
}
