//! `Aspose.Pdf.Annotations.InkAnnotation` — a freehand-ink
//! annotation, per `capabilities/ink_annotation.yaml`.
//!
//! Extends [`MarkupAnnotation`]. The non-generic
//! `System.Collections.IList` constructor is dropped in favour of
//! the typed `Vec<Vec<Point>>` overloads; `Accept` /
//! `ChangeAfterResize` drop too.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::cap_style::CapStyle;
use crate::public::document::Document;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::point::Point;
use crate::public::rectangle::Rectangle;

/// A freehand-ink annotation (a list of point strokes).
pub struct InkAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    cap_style: CapStyle,
    ink_list: Vec<Vec<Point>>,
}

impl InkAnnotation {
    fn build(base: AnnotationBase, ink_list: Vec<Vec<Point>>) -> Self {
        Self {
            base,
            markup: MarkupBase::new(),
            cap_style: CapStyle::Rectangular,
            ink_list,
        }
    }

    /// Construct within a [`Document`] from a list of strokes.
    pub fn new(_document: &Document, ink_list: Vec<Vec<Point>>) -> Self {
        Self::build(AnnotationBase::default(), ink_list)
    }

    /// Construct on a [`Page`] with a bounding rectangle and strokes.
    pub fn on_page(page: &Page, rect: Rectangle, ink_list: Vec<Vec<Point>>) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self::build(base, ink_list)
    }

    /// The stroke cap style.
    pub fn cap_style(&self) -> CapStyle {
        self.cap_style
    }
    /// Set the stroke cap style.
    pub fn set_cap_style(&mut self, value: CapStyle) {
        self.cap_style = value;
    }

    /// The ink strokes (each a polyline of points).
    pub fn ink_list(&self) -> &[Vec<Point>] {
        &self.ink_list
    }
    /// Set the ink strokes.
    pub fn set_ink_list(&mut self, value: Vec<Vec<Point>>) {
        self.ink_list = value;
    }
}

impl HasBase for InkAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        if self.ink_list.is_empty() {
            return Vec::new();
        }
        // §12.5.6.13: one array per stroke, alternating x and y.
        let strokes = self
            .ink_list
            .iter()
            .map(|stroke| stroke.iter().flat_map(|p| [p.x(), p.y()]).collect())
            .collect();
        vec![("InkList", Entry::RealGroups(strokes))]
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}
impl HasMarkupBase for InkAnnotation {
    fn markup(&self) -> &MarkupBase {
        &self.markup
    }
    fn markup_mut(&mut self) -> &mut MarkupBase {
        &mut self.markup
    }
}
impl Annotation for InkAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Ink
    }
}
impl MarkupAnnotation for InkAnnotation {}
