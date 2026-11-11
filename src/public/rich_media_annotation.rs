//! `Aspose.Pdf.Annotations.RichMediaAnnotation` — a rich-media (audio /
//! video) annotation, per `capabilities/rich_media_annotation.yaml`.
//!
//! Extends [`Annotation`] directly. The `Stream`
//! content surfaces (`AddCustomData` / `SetContent` / `SetPoster` /
//! `CustomPlayer` / `Content`) are dropped; the content kind, activation
//! event, and custom Flash variables ship.
//!
//! **Carried, not written.** A rich-media annotation's `/RichMediaContent` is not among the
//! entries this crate emits for an annotation — it writes the subtype, the
//! rectangle, the contents and name, the colour and flags, the markup
//! entries, and the entry that gives a geometry subtype its shape. These
//! properties round-trip in memory.

use crate::public::activation_event::ActivationEvent;
use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::content_type::ContentType;
use crate::public::document::Document;
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::internal::pending::PENDING;

/// A rich-media (audio / video) annotation.
pub struct RichMediaAnnotation {
    base: AnnotationBase,
    content_type: ContentType,
    activate_on: ActivationEvent,
    custom_flash_variables: String,
}

impl RichMediaAnnotation {
    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self {
        Self {
            base: AnnotationBase::default(),
            content_type: ContentType::Audio,
            activate_on: ActivationEvent::Click,
            custom_flash_variables: String::new(),
        }
    }

    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            content_type: ContentType::Audio,
            activate_on: ActivationEvent::Click,
            custom_flash_variables: String::new(),
        }
    }

    /// Re-emit the annotation's stream content.
    ///
    /// Not implemented — pending `annotation_appearance_streams`.
    /// Calling this panics.
    pub fn update(&mut self) {
        PENDING("annotation_appearance_streams")
    }

    /// The media content kind.
    pub fn content_type(&self) -> ContentType {
        self.content_type
    }
    /// Set the media content kind.
    pub fn set_content_type(&mut self, value: ContentType) {
        self.content_type = value;
    }

    /// The activation event.
    pub fn activate_on(&self) -> ActivationEvent {
        self.activate_on
    }
    /// Set the activation event.
    pub fn set_activate_on(&mut self, value: ActivationEvent) {
        self.activate_on = value;
    }

    /// The custom Flash variables string.
    pub fn custom_flash_variables(&self) -> &str {
        &self.custom_flash_variables
    }
    /// Set the custom Flash variables string.
    pub fn set_custom_flash_variables(&mut self, value: impl Into<String>) {
        self.custom_flash_variables = value.into();
    }
}

impl HasBase for RichMediaAnnotation {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
}

impl Annotation for RichMediaAnnotation {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::RichMedia
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_and_accessors() {
        let mut a = RichMediaAnnotation::new(&Document::new());
        assert_eq!(a.annotation_type(), AnnotationType::RichMedia);
        assert_eq!(a.content_type(), ContentType::Audio);
        assert_eq!(a.activate_on(), ActivationEvent::Click);
        a.set_content_type(ContentType::Video);
        a.set_activate_on(ActivationEvent::PageOpen);
        a.set_custom_flash_variables("autostart=1");
        assert_eq!(a.content_type(), ContentType::Video);
        assert_eq!(a.activate_on(), ActivationEvent::PageOpen);
        assert_eq!(a.custom_flash_variables(), "autostart=1");
    }

    #[test]
    #[should_panic(expected = "annotation_appearance_streams")]
    fn update_is_pending() {
        let mut a = RichMediaAnnotation::new(&Document::new());
        a.update();
    }
}
