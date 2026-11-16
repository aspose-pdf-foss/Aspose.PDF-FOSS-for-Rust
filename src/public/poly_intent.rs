//! `Aspose.Pdf.Annotations.PolyIntent` — annotation enum, per `capabilities/poly_intent.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// What a polygon or polyline annotation is being used as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PolyIntent {
    /// No intent; nothing is written.
    #[default]
    Undefined = 0,
    /// A cloud-shaped outline.
    PolygonCloud = 1,
    /// An open dimension line measuring a distance.
    PolyLineDimension = 2,
    /// A closed dimension outline measuring an area.
    PolygonDimension = 3,
}
