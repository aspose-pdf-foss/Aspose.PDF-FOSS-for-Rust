//! `Aspose.Pdf.Annotations.TrimMarkAnnotation` — a trim corner printer
//! mark, per `capabilities/trim_mark_annotation.yaml`.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::corner_printer_mark_annotation::sealed::HasCornerBase;
use crate::public::corner_printer_mark_annotation::{CornerBase, CornerPrinterMarkAnnotation};
use crate::public::page::Page;
use crate::public::printer_mark_annotation::PrinterMarkAnnotation;
use crate::public::printer_mark_corner_position::PrinterMarkCornerPosition;

/// A trim corner printer mark.
pub struct TrimMarkAnnotation {
    base: AnnotationBase,
    corner: CornerBase,
}

impl TrimMarkAnnotation {
    /// Construct on a [`Page`] anchored to the given corner.
    pub fn on_page(page: &Page, position: PrinterMarkCornerPosition) -> Self {
        let base = AnnotationBase {
            page_index: page.number() as i32,
            ..AnnotationBase::default()
        };
        Self { base, corner: CornerBase::new(position) }
    }
}

impl HasBase for TrimMarkAnnotation {
    fn base(&self) -> &AnnotationBase { &self.base }
    fn base_mut(&mut self) -> &mut AnnotationBase { &mut self.base }
}
impl HasCornerBase for TrimMarkAnnotation {
    fn corner(&self) -> &CornerBase { &self.corner }
    fn corner_mut(&mut self) -> &mut CornerBase { &mut self.corner }
}
impl Annotation for TrimMarkAnnotation {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::TrimMark }
}
impl PrinterMarkAnnotation for TrimMarkAnnotation {}
impl CornerPrinterMarkAnnotation for TrimMarkAnnotation {}
