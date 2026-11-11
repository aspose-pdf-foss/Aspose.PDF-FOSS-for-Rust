//! `Aspose.Pdf.Annotations.WatermarkAnnotation` — a watermark
//! annotation, per `capabilities/watermark_annotation.yaml`.
//!
//! Extends [`Annotation`] directly. `SetTextAndState`
//! sets the multi-line watermark text and its styling; `FixedPrint` is
//! the fixed-print placement. `SetText` (which takes `FormattedText`) is
//! dropped (that type is not in the shipped subset).
//!
//! **Carried, not written.** The watermark's text, its styling and its
//! opacity are drawn, not declared: they belong in an appearance stream
//! (`/AP /N`), a second object, and the annotation writer emits one
//! dictionary per annotation. The path that does put watermark text on
//! the page is a page artifact — see
//! [`Page::artifacts`](crate::Page::artifacts), which this crate
//! renders into the content stream on save.
//!
//! `/FixedPrint` (§12.5.6.22) is left unwritten for the same reason and
//! not for want of a place to put it: the dictionary says where to
//! place the *appearance* when printing, so with no appearance stream
//! it would place nothing.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::fixed_print::FixedPrint;
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::text_state::TextState;

/// A watermark annotation.
pub struct WatermarkAnnotation {
    base: AnnotationBase,
    opacity: f64,
    // The watermark text lines + their styling are recorded by
    // set_text_and_state and consumed by render-on-save (a follow-on);
    // there is no canonical getter, so they are not read back here.
    #[allow(dead_code)]
    text: Vec<String>,
    #[allow(dead_code)]
    text_state: TextState,
    fixed_print: FixedPrint,
}

impl WatermarkAnnotation {
    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            opacity: 1.0,
            text: Vec::new(),
            text_state: TextState::default(),
            fixed_print: FixedPrint::default(),
        }
    }

    /// Set the watermark text lines together with their styling.
    pub fn set_text_and_state(&mut self, text: Vec<String>, text_state: TextState) {
        self.text = text;
        self.text_state = text_state;
    }

    /// The watermark opacity (`0.0`–`1.0`).
    pub fn opacity(&self) -> f64 {
        self.opacity
    }
    /// Set the watermark opacity.
    pub fn set_opacity(&mut self, value: f64) {
        self.opacity = value;
    }

    /// The fixed-print placement of the watermark.
    pub fn fixed_print(&self) -> &FixedPrint {
        &self.fixed_print
    }
    /// The fixed-print placement, mutably. The canonical member is a
    /// getter returning an object a caller mutates through; Rust needs
    /// a second accessor to offer the same reach, the way
    /// [`Document::form`](crate::Document::form) is paired with
    /// `form_mut`.
    pub fn fixed_print_mut(&mut self) -> &mut FixedPrint {
        &mut self.fixed_print
    }
}

impl HasBase for WatermarkAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
}
impl Annotation for WatermarkAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Watermark
    }
}
