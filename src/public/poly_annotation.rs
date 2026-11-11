//! `Aspose.Pdf.Annotations.PolyAnnotation` — the intermediate base
//! for polygon / polyline annotations, per
//! `capabilities/poly_annotation.yaml`.
//!
//! Modelled as the [`PolyAnnotation`] trait (extending
//! [`MarkupAnnotation`]) over a shared `PolyBase`. `Measure`
//! (`Aspose.Pdf.Annotations.Measure`) and `ChangeAfterResize`
//! (`Aspose.Pdf.Matrix`) are dropped.

use crate::public::color::Color;
use crate::public::line_ending::LineEnding;
use crate::public::markup_annotation::MarkupAnnotation;
use crate::public::measure::Measure;
use crate::public::point::Point;
use crate::public::poly_intent::PolyIntent;

/// Shared polygon/polyline fields. Crate-internal.
pub(crate) struct PolyBase {
    pub(crate) vertices: Vec<Point>,
    pub(crate) interior_color: Color,
    pub(crate) starting_style: LineEnding,
    pub(crate) ending_style: LineEnding,
    pub(crate) intent: PolyIntent,
    pub(crate) measure: Measure,
}

impl PolyBase {
    pub(crate) fn new(vertices: Vec<Point>) -> Self {
        Self {
            vertices,
            interior_color: Color::empty(),
            starting_style: LineEnding::None,
            ending_style: LineEnding::None,
            intent: PolyIntent::Undefined,
            measure: Measure::default(),
        }
    }
}

pub(crate) mod sealed {
    use super::PolyBase;

    /// Poly-base access for concrete polygon/polyline annotations.
    pub(crate) trait HasPolyBase {
        /// The shared poly base.
        fn poly(&self) -> &PolyBase;
        /// The shared poly base, mutably.
        fn poly_mut(&mut self) -> &mut PolyBase;
    }
}

/// A polygon or polyline annotation defined by its vertices.
#[allow(private_bounds)]
pub trait PolyAnnotation: MarkupAnnotation + sealed::HasPolyBase {
    /// The vertices.
    fn vertices(&self) -> &[Point] {
        &self.poly().vertices
    }
    /// Set the vertices.
    fn set_vertices(&mut self, value: Vec<Point>) {
        self.poly_mut().vertices = value;
    }

    /// The interior fill colour.
    fn interior_color(&self) -> Color {
        self.poly().interior_color
    }
    /// Set the interior fill colour.
    fn set_interior_color(&mut self, value: Color) {
        self.poly_mut().interior_color = value;
    }

    /// The starting line-ending style.
    fn starting_style(&self) -> LineEnding {
        self.poly().starting_style
    }
    /// Set the starting line-ending style.
    fn set_starting_style(&mut self, value: LineEnding) {
        self.poly_mut().starting_style = value;
    }

    /// The ending line-ending style.
    fn ending_style(&self) -> LineEnding {
        self.poly().ending_style
    }
    /// Set the ending line-ending style.
    fn set_ending_style(&mut self, value: LineEnding) {
        self.poly_mut().ending_style = value;
    }

    /// The poly intent (cloud / dimension).
    fn intent(&self) -> PolyIntent {
        self.poly().intent
    }
    /// Set the poly intent.
    fn set_intent(&mut self, value: PolyIntent) {
        self.poly_mut().intent = value;
    }

    /// The measurement parameters (canonical `Measure`).
    fn measure(&self) -> &Measure {
        &self.poly().measure
    }
    /// Set the measurement parameters.
    fn set_measure(&mut self, value: Measure) {
        self.poly_mut().measure = value;
    }
}

/// The name PDF gives a poly annotation's intent (§12.5.6.9). An
/// undefined intent is the file saying nothing, so it has no name.
pub(crate) fn poly_intent_name(i: PolyIntent) -> Option<&'static str> {
    match i {
        PolyIntent::Undefined => None,
        PolyIntent::PolygonCloud => Some("PolygonCloud"),
        PolyIntent::PolyLineDimension => Some("PolyLineDimension"),
        PolyIntent::PolygonDimension => Some("PolygonDimension"),
    }
}
