//! `Aspose.Pdf.Annotations.ExplicitDestinationType` — the fit kind
//! of an explicit destination, per
//! `capabilities/explicit_destination_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Annotations.ExplicitDestinationType` enum.

/// The fit kind of an explicit destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ExplicitDestinationType {
    /// Fit to a point + zoom (`/XYZ`).
    #[default]
    XYZ = 0,
    /// Fit the whole page (`/Fit`).
    Fit = 1,
    /// Fit the page width at a given top (`/FitH`).
    FitH = 2,
    /// Fit the page height at a given left (`/FitV`).
    FitV = 3,
    /// Fit a rectangle (`/FitR`).
    FitR = 4,
    /// Fit the bounding box (`/FitB`).
    FitB = 5,
    /// Fit the bounding-box width at a given top (`/FitBH`).
    FitBH = 6,
    /// Fit the bounding-box height at a given left (`/FitBV`).
    FitBV = 7,
}
