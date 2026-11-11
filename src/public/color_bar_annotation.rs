//! `Aspose.Pdf.Annotations.ColorBarAnnotation` — a colour-bar
//! printer mark, per `capabilities/color_bar_annotation.yaml`.
//!
//! **Carried, not written.** A colour-bar printer mark's plate selection is not among the
//! entries this crate emits for an annotation — it writes the subtype, the
//! rectangle, the contents and name, the colour and flags, the markup
//! entries, and the entry that gives a geometry subtype its shape. These
//! properties round-trip in memory.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::colors_of_cmyk::ColorsOfCMYK;
use crate::public::page::Page;
use crate::public::printer_mark_annotation::PrinterMarkAnnotation;
use crate::public::rectangle::Rectangle;

/// A colour-bar printer mark for one process colour.
pub struct ColorBarAnnotation {
    base: AnnotationBase,
    color_of_cmyk: ColorsOfCMYK,
}

impl ColorBarAnnotation {
    /// Construct on a [`Page`] at the given rectangle for one CMYK
    /// process colour.
    pub fn on_page(page: &Page, rect: Rectangle, color_of_cmyk: ColorsOfCMYK) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            color_of_cmyk,
        }
    }

    /// The process colour this bar represents.
    pub fn color_of_cmyk(&self) -> ColorsOfCMYK {
        self.color_of_cmyk
    }
    /// Set the process colour.
    pub fn set_color_of_cmyk(&mut self, value: ColorsOfCMYK) {
        self.color_of_cmyk = value;
    }
}

impl HasBase for ColorBarAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
}
impl Annotation for ColorBarAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::ColorBar
    }
}
impl PrinterMarkAnnotation for ColorBarAnnotation {}
