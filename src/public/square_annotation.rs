//! `Aspose.Pdf.Annotations.SquareAnnotation` — a square (rectangle-bounded
//! ellipse / box) figure annotation, per `capabilities/square_annotation.yaml`.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::common_figure_annotation::sealed::HasFigureBase;
use crate::public::common_figure_annotation::{CommonFigureAnnotation, CommonFigureBase};
use crate::public::document::Document;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;

/// A square figure annotation.
pub struct SquareAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    figure: CommonFigureBase,
}

impl SquareAnnotation {
    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self {
        Self {
            base: AnnotationBase::default(),
            markup: MarkupBase::new(),
            figure: CommonFigureBase::new(),
        }
    }

    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            markup: MarkupBase::new(),
            figure: CommonFigureBase::new(),
        }
    }
}

impl HasBase for SquareAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn figure_base(
        &self,
    ) -> Option<&crate::public::common_figure_annotation::CommonFigureBase> {
        Some(&self.figure)
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}

impl HasMarkupBase for SquareAnnotation {
    fn markup(&self) -> &MarkupBase {
        &self.markup
    }
    fn markup_mut(&mut self) -> &mut MarkupBase {
        &mut self.markup
    }
}

impl HasFigureBase for SquareAnnotation {
    fn figure(&self) -> &CommonFigureBase {
        &self.figure
    }
    fn figure_mut(&mut self) -> &mut CommonFigureBase {
        &mut self.figure
    }
}

impl Annotation for SquareAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Square
    }
}

impl MarkupAnnotation for SquareAnnotation {}
impl CommonFigureAnnotation for SquareAnnotation {}
