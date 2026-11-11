//! `Aspose.Pdf.Annotations.LinkAnnotation` — a hyperlink / GoTo
//! annotation, per `capabilities/link_annotation.yaml`.
//!
//! Extends [`Annotation`] directly (it carries no
//! author markup). Holds an optional action and destination plus the
//! link highlighting mode.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::highlighting_mode::HighlightingMode;
use crate::public::i_appointment::IAppointment;
use crate::public::page::Page;
use crate::public::pdf_action::PdfAction;
use crate::public::rectangle::Rectangle;

/// A link annotation (a clickable region with an action or
/// destination).
pub struct LinkAnnotation {
    base: AnnotationBase,
    action: Option<Box<dyn PdfAction>>,
    destination: Option<Box<dyn IAppointment>>,
    highlighting: HighlightingMode,
}

impl LinkAnnotation {
    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            action: None,
            destination: None,
            highlighting: HighlightingMode::None,
        }
    }

    /// The action triggered when the link is activated, if any.
    pub fn action(&self) -> Option<&dyn PdfAction> {
        self.action.as_deref()
    }
    /// Set the action triggered when the link is activated.
    pub fn set_action(&mut self, action: Box<dyn PdfAction>) {
        self.action = Some(action);
    }

    /// The destination jumped to when the link is activated, if any.
    pub fn destination(&self) -> Option<&dyn IAppointment> {
        self.destination.as_deref()
    }
    /// Set the destination jumped to when the link is activated.
    pub fn set_destination(&mut self, destination: Box<dyn IAppointment>) {
        self.destination = Some(destination);
    }

    /// The link highlighting mode.
    pub fn highlighting(&self) -> HighlightingMode {
        self.highlighting
    }
    /// Set the link highlighting mode.
    pub fn set_highlighting(&mut self, value: HighlightingMode) {
        self.highlighting = value;
    }
}

impl HasBase for LinkAnnotation {
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

impl Annotation for LinkAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Link
    }
}
