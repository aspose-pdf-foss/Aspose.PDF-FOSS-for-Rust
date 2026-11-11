//! `Aspose.Pdf.Annotations.PolygonAnnotation` — a polygon annotation, per
//! `capabilities/polygon_annotation.yaml`.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::document::Document;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::point::Point;
use crate::public::poly_annotation::sealed::HasPolyBase;
use crate::public::poly_annotation::{PolyAnnotation, PolyBase};
use crate::public::rectangle::Rectangle;

/// A polygon annotation defined by its vertices.
pub struct PolygonAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    poly: PolyBase,
}

impl PolygonAnnotation {
    /// Construct within a [`Document`] from the given vertices.
    pub fn new(_document: &Document, vertices: Vec<Point>) -> Self {
        Self {
            base: AnnotationBase::default(),
            markup: MarkupBase::new(),
            poly: PolyBase::new(vertices),
        }
    }

    /// Construct on a [`Page`] with a bounding rectangle and vertices.
    pub fn on_page(page: &Page, rect: Rectangle, vertices: Vec<Point>) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            markup: MarkupBase::new(),
            poly: PolyBase::new(vertices),
        }
    }
}

impl HasBase for PolygonAnnotation {
    fn base(&self) -> &AnnotationBase { &self.base }
    fn base_mut(&mut self) -> &mut AnnotationBase { &mut self.base }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.9: /Vertices alternates x and y; without it the shape
        // has no outline for a viewer to draw.
        let mut out = Vec::new();
        if !self.poly.vertices.is_empty() {
            let flat = self.poly.vertices.iter().flat_map(|p| [p.x(), p.y()]).collect();
            out.push(("Vertices", Entry::Reals(flat)));
        }
        if !self.poly.interior_color.is_empty() {
            let (r, g, b) = self.poly.interior_color.rgb();
            out.push(("IC", Entry::Reals(vec![r, g, b])));
        }
        if self.poly.starting_style != crate::public::line_ending::LineEnding::None
            || self.poly.ending_style != crate::public::line_ending::LineEnding::None
        {
            out.push((
                "LE",
                Entry::Names(vec![
                    crate::public::line_annotation::line_ending_name(self.poly.starting_style)
                        .to_string(),
                    crate::public::line_annotation::line_ending_name(self.poly.ending_style)
                        .to_string(),
                ]),
            ));
        }
        if let Some(intent) = crate::public::poly_annotation::poly_intent_name(self.poly.intent) {
            out.push(("IT", Entry::Name(intent.to_string())));
        }
        // §12.9's measurement parameters, as the plain dictionary
        // Table 261 defines — written only when they say something.
        if let Some(measure) = crate::public::measure::measure_entries(&self.poly.measure) {
            out.push(("Measure", measure));
        }
        out
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}
impl HasMarkupBase for PolygonAnnotation {
    fn markup(&self) -> &MarkupBase { &self.markup }
    fn markup_mut(&mut self) -> &mut MarkupBase { &mut self.markup }
}
impl HasPolyBase for PolygonAnnotation {
    fn poly(&self) -> &PolyBase { &self.poly }
    fn poly_mut(&mut self) -> &mut PolyBase { &mut self.poly }
}
impl Annotation for PolygonAnnotation {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::Polygon }
}
impl MarkupAnnotation for PolygonAnnotation {}
impl PolyAnnotation for PolygonAnnotation {}
