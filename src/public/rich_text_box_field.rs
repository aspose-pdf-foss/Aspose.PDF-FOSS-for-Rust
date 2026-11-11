//! `Aspose.Pdf.Forms.RichTextBoxField` — a rich-text text-box form
//! field, per `capabilities/rich_text_box_field.yaml`.
//!
//! Extends [`TextBoxField`] (Rust models the inheritance by embedding a
//! base `TextBoxField` + [`Deref`], so the inherited text-box surface
//! applies here too).
//!
//! `/RV`, `/DS` and `/Q` are written on save (§12.7.4.3), together with
//! the RichText flag that makes a viewer read them.
//!
//! **Carried, not written.** `formatted_value` is what a viewer shows
//! after the field's format script has run; the format script is the
//! thing a file carries, and this crate writes none, so the formatted
//! result has no entry to go into.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::justification::Justification;
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::text_box_field::TextBoxField;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;
use std::ops::{Deref, DerefMut};

/// A rich-text text-box form field.
pub struct RichTextBoxField {
    base: TextBoxField,
    style: String,
    rich_text_value: String,
    formatted_value: String,
    justify: Justification,
}

impl RichTextBoxField {
    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self { base: TextBoxField::build(base),
               style: String::new(), rich_text_value: String::new(),
               formatted_value: String::new(), justify: Justification::Left }
    }
    /// The rich-text default style string.
    pub fn style(&self) -> &str { &self.style }
    /// Set the rich-text default style string.
    pub fn set_style(&mut self, value: String) { self.style = value; }
    /// The rich-text (RTF / XML) value.
    pub fn rich_text_value(&self) -> &str { &self.rich_text_value }
    /// Set the rich-text value.
    pub fn set_rich_text_value(&mut self, value: String) { self.rich_text_value = value; }
    /// The formatted (plain-text) value.
    pub fn formatted_value(&self) -> &str { &self.formatted_value }
    /// Set the formatted value.
    pub fn set_formatted_value(&mut self, value: String) { self.formatted_value = value; }
    /// The text justification.
    pub fn justify(&self) -> Justification { self.justify }
    /// Set the text justification.
    pub fn set_justify(&mut self, value: Justification) { self.justify = value; }
}

impl Deref for RichTextBoxField {
    type Target = TextBoxField;
    fn deref(&self) -> &TextBoxField { &self.base }
}
impl DerefMut for RichTextBoxField {
    fn deref_mut(&mut self) -> &mut TextBoxField { &mut self.base }
}

// The inherited bases live in the embedded TextBoxField, so the field
// and its base read and write the same state.
impl HasBase for RichTextBoxField {
    fn base(&self) -> &AnnotationBase { self.base.base() }
    fn base_mut(&mut self) -> &mut AnnotationBase { self.base.base_mut() }
}
impl HasWidgetBase for RichTextBoxField {
    fn widget(&self) -> &WidgetBase { self.base.widget() }
    fn widget_mut(&mut self) -> &mut WidgetBase { self.base.widget_mut() }
}
impl HasFieldBase for RichTextBoxField {
    fn field(&self) -> &FieldBase { self.base.field() }
    fn field_mut(&mut self) -> &mut FieldBase { self.base.field_mut() }
    fn pdf_field_type(&self) -> &'static [u8] { self.base.pdf_field_type() }
    fn pdf_field_flags(&self) -> i64 {
        // The embedded text box owns multiline, comb and the rest of
        // Table 228; without forwarding, a rich text box lost every
        // one of them. RichText is bit 26 and is what makes a viewer
        // read /RV instead of /V.
        self.base.pdf_field_flags() | (1 << 25)
    }
    fn pdf_field_entries(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // /MaxLen and anything else the text box contributes, then
        // §12.7.4.3: /RV is the rich-text value a viewer renders, /DS
        // the default style it renders it with, /Q the quadding
        // (0 left, 1 centred, 2 right) applied to the lines.
        let mut out = self.base.pdf_field_entries();
        if !self.rich_text_value.is_empty() {
            out.push(("RV", Entry::Text(self.rich_text_value.clone())));
        }
        if !self.style.is_empty() {
            out.push(("DS", Entry::Text(self.style.clone())));
        }
        if self.justify != Justification::Left {
            out.push(("Q", Entry::Int(self.justify as i64)));
        }
        out
    }
}
impl Annotation for RichTextBoxField {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::Widget }
}
impl Field for RichTextBoxField {}
