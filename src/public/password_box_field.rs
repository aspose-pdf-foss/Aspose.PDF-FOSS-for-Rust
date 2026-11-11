//! `Aspose.Pdf.Forms.PasswordBoxField` — a password text-box form
//! field, per `capabilities/password_box_field.yaml`.
//!
//! A password field extends [`TextBoxField`]
//! with no additional members of its own; it carries the inherited
//! [`Field`] surface.
//!
//! Extends [`TextBoxField`] (Rust models the inheritance by embedding a
//! base `TextBoxField` + [`Deref`], so the inherited text-box surface
//! applies here too).

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::document::Document;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::text_box_field::TextBoxField;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;
use std::ops::{Deref, DerefMut};

/// A password text-box form field.
pub struct PasswordBoxField {
    base: TextBoxField,
}

impl PasswordBoxField {
    fn build(base: AnnotationBase) -> Self {
        Self { base: TextBoxField::build(base) }
    }
    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self { Self::build(AnnotationBase::default()) }
    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self::build(base)
    }
}

impl Deref for PasswordBoxField {
    type Target = TextBoxField;
    fn deref(&self) -> &TextBoxField { &self.base }
}
impl DerefMut for PasswordBoxField {
    fn deref_mut(&mut self) -> &mut TextBoxField { &mut self.base }
}

// The inherited bases live in the embedded TextBoxField, so the field
// and its base read and write the same state.
impl HasBase for PasswordBoxField {
    fn base(&self) -> &AnnotationBase { self.base.base() }
    fn base_mut(&mut self) -> &mut AnnotationBase { self.base.base_mut() }
}
impl HasWidgetBase for PasswordBoxField {
    fn widget(&self) -> &WidgetBase { self.base.widget() }
    fn widget_mut(&mut self) -> &mut WidgetBase { self.base.widget_mut() }
}
impl HasFieldBase for PasswordBoxField {
    fn field(&self) -> &FieldBase { self.base.field() }
    fn field_mut(&mut self) -> &mut FieldBase { self.base.field_mut() }
    fn pdf_field_flags(&self) -> i64 {
        // Table 228: Password is bit 14. Without it a viewer echoes
        // what is typed.
        (1 << 13) | self.base.pdf_field_flags()
    }
    fn pdf_field_entries(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        self.base.pdf_field_entries()
    }
}
impl Annotation for PasswordBoxField {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::Widget }
}
impl Field for PasswordBoxField {}
