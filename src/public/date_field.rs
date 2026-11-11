//! `Aspose.Pdf.Forms.DateField` — a date text-box form field, per
//! `capabilities/date_field.yaml`.
//!
//! The `AddImage` (`Aspose.Pdf.Image`) override is dropped. `Value`
//! (`System.DateTime`) ships; the inherited [`Field`]
//! surface carries the string value.
//!
//! Extends [`TextBoxField`] (Rust models the inheritance by embedding a
//! base `TextBoxField` + [`Deref`], so the inherited text-box surface
//! applies here too).

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::date_time::DateTime;
use crate::public::document::Document;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::text_box_field::TextBoxField;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;
use std::ops::{Deref, DerefMut};
use crate::internal::pending::PENDING;

/// A date text-box form field.
pub struct DateField {
    base: TextBoxField,
    date_format: String,
    value: DateTime,
}

impl DateField {
    fn build(base: AnnotationBase) -> Self {
        Self { base: TextBoxField::build(base),
               date_format: String::new(), value: DateTime::default() }
    }
    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self { Self::build(AnnotationBase::default()) }
    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self::build(base)
    }
    /// Initialise the field on a [`Page`].
    ///
    /// Not implemented — pending `form_field_page_binding`. Calling
    /// this panics.
    pub fn init(&mut self, _page: &Page) {
        PENDING("form_field_page_binding")
    }
    /// The date format string (e.g. `mm/dd/yyyy`).
    pub fn date_format(&self) -> &str { &self.date_format }
    /// Set the date format string.
    pub fn set_date_format(&mut self, value: String) { self.date_format = value; }
    /// The date value (canonical `Value`).
    pub fn value(&self) -> DateTime { self.value }
    /// Set the date value.
    pub fn set_value(&mut self, value: DateTime) { self.value = value; }
}

impl Deref for DateField {
    type Target = TextBoxField;
    fn deref(&self) -> &TextBoxField { &self.base }
}
impl DerefMut for DateField {
    fn deref_mut(&mut self) -> &mut TextBoxField { &mut self.base }
}

// The inherited bases live in the embedded TextBoxField, so the field
// and its base read and write the same state.
impl HasBase for DateField {
    fn base(&self) -> &AnnotationBase { self.base.base() }
    fn base_mut(&mut self) -> &mut AnnotationBase { self.base.base_mut() }
}
impl HasWidgetBase for DateField {
    fn widget(&self) -> &WidgetBase { self.base.widget() }
    fn widget_mut(&mut self) -> &mut WidgetBase { self.base.widget_mut() }
}
impl HasFieldBase for DateField {
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
impl Annotation for DateField {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::Widget }
}
impl Field for DateField {}
