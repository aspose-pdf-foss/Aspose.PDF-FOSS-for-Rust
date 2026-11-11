//! `Aspose.Pdf.Forms.NumberField` — a numeric text-box form field,
//! per `capabilities/number_field.yaml`.
//!
//! Extends [`TextBoxField`] (Rust models the inheritance by embedding a
//! base `TextBoxField` + [`Deref`], so the inherited text-box surface
//! applies here too).
//!
//! **Carried, not written.** `allowed_chars` restricts typing, which a
//! PDF expresses as a keystroke JavaScript action rather than a field
//! entry; this crate writes no such action, so the restriction stays in
//! memory.

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

/// A numeric text-box form field.
pub struct NumberField {
    base: TextBoxField,
    allowed_chars: String,
}

impl NumberField {
    fn build(base: AnnotationBase) -> Self {
        Self { base: TextBoxField::build(base), allowed_chars: String::new() }
    }
    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self::build(base)
    }
    /// Construct within a [`Document`] at the given rectangle.
    pub fn for_document(_document: &Document, rect: Rectangle) -> Self {
        Self::build(AnnotationBase::new(rect))
    }
    /// The set of characters accepted by the field.
    pub fn allowed_chars(&self) -> &str { &self.allowed_chars }
    /// Set the set of accepted characters.
    pub fn set_allowed_chars(&mut self, value: String) { self.allowed_chars = value; }
}

impl Deref for NumberField {
    type Target = TextBoxField;
    fn deref(&self) -> &TextBoxField { &self.base }
}
impl DerefMut for NumberField {
    fn deref_mut(&mut self) -> &mut TextBoxField { &mut self.base }
}

// The inherited bases live in the embedded TextBoxField, so the field
// and its base read and write the same state.
impl HasBase for NumberField {
    fn base(&self) -> &AnnotationBase { self.base.base() }
    fn base_mut(&mut self) -> &mut AnnotationBase { self.base.base_mut() }
}
impl HasWidgetBase for NumberField {
    fn widget(&self) -> &WidgetBase { self.base.widget() }
    fn widget_mut(&mut self) -> &mut WidgetBase { self.base.widget_mut() }
}
impl HasFieldBase for NumberField {
    fn field(&self) -> &FieldBase { self.base.field() }
    fn field_mut(&mut self) -> &mut FieldBase { self.base.field_mut() }
    // The embedded text box owns the Table 228 bits and /MaxLen; a
    // descendant that does not forward them writes a field stripped of
    // everything its own base was told.
    fn pdf_field_flags(&self) -> i64 { self.base.pdf_field_flags() }
    fn pdf_field_entries(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        self.base.pdf_field_entries()
    }
}
impl Annotation for NumberField {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::Widget }
}
impl Field for NumberField {}
