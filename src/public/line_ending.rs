//! `Aspose.Pdf.Annotations.LineEnding` — annotation enum, per `capabilities/line_ending.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The shape drawn at an end of a line, polyline or callout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum LineEnding {
    /// No special ending; the line simply stops.
    #[default]
    None = 0,
    /// A filled square.
    Square = 1,
    /// A filled circle.
    Circle = 2,
    /// A filled diamond.
    Diamond = 3,
    /// Two short lines meeting in an open arrowhead.
    OpenArrow = 4,
    /// A filled triangular arrowhead.
    ClosedArrow = 5,
    /// A short line at right angles to the line itself.
    Butt = 6,
    /// An open arrowhead pointing the other way.
    ROpenArrow = 7,
    /// A filled arrowhead pointing the other way.
    RClosedArrow = 8,
    /// A short line angled across the end.
    Slash = 9,
}
