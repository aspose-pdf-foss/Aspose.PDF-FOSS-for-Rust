//! `Aspose.Pdf.Annotations.PopupAnnotation` — a pop-up note window
//! for a parent markup annotation, per
//! `capabilities/popup_annotation.yaml`.
//!
//! Extends [`Annotation`] directly (it carries no
//! author markup). The `Parent` is the markup annotation this pop-up
//! belongs to.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::document::Document;
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;

/// A pop-up note window.
pub struct PopupAnnotation {
    base: AnnotationBase,
    open: bool,
    parent: Option<Box<dyn Annotation>>,
}

impl PopupAnnotation {
    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self {
        Self {
            base: AnnotationBase::default(),
            open: false,
            parent: None,
        }
    }

    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            open: false,
            parent: None,
        }
    }

    /// Whether the pop-up is open by default.
    pub fn open(&self) -> bool {
        self.open
    }
    /// Set whether the pop-up is open by default.
    pub fn set_open(&mut self, value: bool) {
        self.open = value;
    }

    /// The parent markup annotation, if set.
    pub fn parent(&self) -> Option<&dyn Annotation> {
        self.parent.as_deref()
    }
    /// Set the parent markup annotation.
    pub fn set_parent(&mut self, parent: Box<dyn Annotation>) {
        self.parent = Some(parent);
    }
}

impl HasBase for PopupAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
    fn pdf_entries(&self) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.14: /Open says whether the pop-up starts displayed.
        if self.open {
            vec![("Open", Entry::Bool(true))]
        } else {
            Vec::new()
        }
    }
}
impl Annotation for PopupAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Popup
    }
}
