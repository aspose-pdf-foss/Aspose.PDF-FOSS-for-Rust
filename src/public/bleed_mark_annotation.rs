//! `Aspose.Pdf.Annotations.BleedMarkAnnotation` — a bleed corner printer
//! mark, per `capabilities/bleed_mark_annotation.yaml`.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::corner_printer_mark_annotation::sealed::HasCornerBase;
use crate::public::corner_printer_mark_annotation::{CornerBase, CornerPrinterMarkAnnotation};
use crate::public::page::Page;
use crate::public::printer_mark_annotation::PrinterMarkAnnotation;
use crate::public::printer_mark_corner_position::PrinterMarkCornerPosition;

/// A bleed corner printer mark.
pub struct BleedMarkAnnotation {
    base: AnnotationBase,
    corner: CornerBase,
}

impl BleedMarkAnnotation {
    /// Construct on a [`Page`] anchored to the given corner.
    pub fn on_page(page: &Page, position: PrinterMarkCornerPosition) -> Self {
        let base = AnnotationBase {
            page_index: page.number() as i32,
            ..AnnotationBase::default()
        };
        Self { base, corner: CornerBase::new(position) }
    }
}

impl HasBase for BleedMarkAnnotation {
    fn base(&self) -> &AnnotationBase { &self.base }
    fn base_mut(&mut self) -> &mut AnnotationBase { &mut self.base }
}
impl HasCornerBase for BleedMarkAnnotation {
    fn corner(&self) -> &CornerBase { &self.corner }
    fn corner_mut(&mut self) -> &mut CornerBase { &mut self.corner }
}
impl Annotation for BleedMarkAnnotation {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::BleedMark }
}
impl PrinterMarkAnnotation for BleedMarkAnnotation {}
impl CornerPrinterMarkAnnotation for BleedMarkAnnotation {}
