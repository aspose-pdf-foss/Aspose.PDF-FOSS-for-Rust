//! `Aspose.Pdf.Annotations.ScreenAnnotation` — a screen (rendition)
//! annotation, per `capabilities/screen_annotation.yaml`.
//!
//! Extends [`Annotation`] directly. `Action` is a
//! read-only rendition action (populated by the engine).

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::page::Page;
use crate::public::pdf_action::PdfAction;
use crate::public::rectangle::Rectangle;

/// A screen annotation that plays a media rendition.
pub struct ScreenAnnotation {
    base: AnnotationBase,
    title: String,
    action: Option<Box<dyn PdfAction>>,
}

impl ScreenAnnotation {
    /// Construct on a [`Page`] at the given rectangle for the media
    /// file at `_media_file`.
    pub fn on_page(page: &Page, rect: Rectangle, _media_file: String) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            title: String::new(),
            action: None,
        }
    }

    /// The screen title.
    pub fn title(&self) -> &str {
        &self.title
    }
    /// Set the screen title.
    pub fn set_title(&mut self, value: String) {
        self.title = value;
    }

    /// The rendition action, if any.
    pub fn action(&self) -> Option<&dyn PdfAction> {
        self.action.as_deref()
    }
}

impl HasBase for ScreenAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        // §12.5.6.19 / §12.6: the action an activated annotation runs.
        // Without /A a link is a rectangle a viewer draws nothing for
        // and does nothing with.
        match self.action.as_deref().and_then(crate::public::pdf_action::action_entry) {
            Some(entry) => vec![("A", entry)],
            None => Vec::new(),
        }
    }
}
impl Annotation for ScreenAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Screen
    }
}
