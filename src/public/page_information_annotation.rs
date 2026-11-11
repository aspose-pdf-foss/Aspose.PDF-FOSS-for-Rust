//! `Aspose.Pdf.Annotations.PageInformationAnnotation` — a
//! page-information printer mark, per
//! `capabilities/page_information_annotation.yaml`.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::page::Page;
use crate::public::printer_mark_annotation::PrinterMarkAnnotation;
use crate::public::rectangle::Rectangle;

/// A page-information printer mark (file name / date / page number).
pub struct PageInformationAnnotation {
    base: AnnotationBase,
}

impl PageInformationAnnotation {
    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self { base }
    }
}

impl HasBase for PageInformationAnnotation {
    fn base(&self) -> &AnnotationBase { &self.base }
    fn base_mut(&mut self) -> &mut AnnotationBase { &mut self.base }
}
impl Annotation for PageInformationAnnotation {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::PageInformation }
}
impl PrinterMarkAnnotation for PageInformationAnnotation {}
