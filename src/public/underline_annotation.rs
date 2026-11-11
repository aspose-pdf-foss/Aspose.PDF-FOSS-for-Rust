//! `Aspose.Pdf.Annotations.UnderlineAnnotation` — a underline text-markup
//! annotation, per `capabilities/underline_annotation.yaml`.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::markup_annotation::sealed::HasMarkupBase;
use crate::public::markup_annotation::{MarkupAnnotation, MarkupBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::text_markup_annotation::sealed::HasTextMarkupBase;
use crate::public::text_markup_annotation::{TextMarkupAnnotation, TextMarkupBase};

/// A underline text-markup annotation.
pub struct UnderlineAnnotation {
    base: AnnotationBase,
    markup: MarkupBase,
    text_markup: TextMarkupBase,
}

impl UnderlineAnnotation {
    /// Construct on a [`Page`] over the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            markup: MarkupBase::new(),
            text_markup: TextMarkupBase::new(),
        }
    }
}

impl HasBase for UnderlineAnnotation {
    fn base(&self) -> &AnnotationBase { &self.base }
    fn base_mut(&mut self) -> &mut AnnotationBase { &mut self.base }
    fn text_markup_base(
        &self,
    ) -> Option<&crate::public::text_markup_annotation::TextMarkupBase> {
        Some(&self.text_markup)
    }
    fn markup_base(&self) -> Option<&crate::public::markup_annotation::MarkupBase> {
        Some(&self.markup)
    }
}
impl HasMarkupBase for UnderlineAnnotation {
    fn markup(&self) -> &MarkupBase { &self.markup }
    fn markup_mut(&mut self) -> &mut MarkupBase { &mut self.markup }
}
impl HasTextMarkupBase for UnderlineAnnotation {
    fn text_markup(&self) -> &TextMarkupBase { &self.text_markup }
    fn text_markup_mut(&mut self) -> &mut TextMarkupBase { &mut self.text_markup }
}
impl Annotation for UnderlineAnnotation {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::Underline }
}
impl MarkupAnnotation for UnderlineAnnotation {}
impl TextMarkupAnnotation for UnderlineAnnotation {}
