//! `Aspose.Pdf.Annotations.RegistrationMarkAnnotation` — a
//! registration printer mark, per
//! `capabilities/registration_mark_annotation.yaml`.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::page::Page;
use crate::public::printer_mark_annotation::PrinterMarkAnnotation;
use crate::public::printer_mark_side_position::PrinterMarkSidePosition;

/// A registration printer mark anchored to a page side.
pub struct RegistrationMarkAnnotation {
    base: AnnotationBase,
    position: PrinterMarkSidePosition,
}

impl RegistrationMarkAnnotation {
    /// Construct on a [`Page`] anchored to the given side.
    pub fn on_page(page: &Page, position: PrinterMarkSidePosition) -> Self {
        let base = AnnotationBase {
            page_index: page.number() as i32,
            ..AnnotationBase::default()
        };
        Self { base, position }
    }

    /// The page side the mark is anchored to.
    pub fn position(&self) -> PrinterMarkSidePosition {
        self.position
    }
    /// Set the page side the mark is anchored to.
    pub fn set_position(&mut self, value: PrinterMarkSidePosition) {
        self.position = value;
    }
}

impl HasBase for RegistrationMarkAnnotation {
    fn base(&self) -> &AnnotationBase { &self.base }
    fn base_mut(&mut self) -> &mut AnnotationBase { &mut self.base }
}
impl Annotation for RegistrationMarkAnnotation {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::RegistrationMark }
}
impl PrinterMarkAnnotation for RegistrationMarkAnnotation {}
