//! `Aspose.Pdf.Annotations.PolyIntent` — annotation enum, per `capabilities/poly_intent.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `PolyIntent` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PolyIntent {
    /// `Undefined`.
    #[default]
    Undefined = 0,
    /// `PolygonCloud`.
    PolygonCloud = 1,
    /// `PolyLineDimension`.
    PolyLineDimension = 2,
    /// `PolygonDimension`.
    PolygonDimension = 3,
}
