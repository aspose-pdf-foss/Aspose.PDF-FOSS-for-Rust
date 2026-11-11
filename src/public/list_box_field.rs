//! `Aspose.Pdf.Forms.ListBoxField` — a list-box choice field, per
//! `capabilities/list_box_field.yaml`.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::choice_field::sealed::HasChoiceBase;
use crate::public::choice_field::{choice_flags, ChoiceBase, ChoiceField};
use crate::public::document::Document;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;

/// A list-box choice field.
pub struct ListBoxField {
    base: AnnotationBase,
    widget: WidgetBase,
    field: FieldBase,
    choice: ChoiceBase,
    top_index: i32,
}

impl ListBoxField {
    fn build(base: AnnotationBase) -> Self {
        Self { base, widget: WidgetBase::new(), field: FieldBase::new(),
               choice: ChoiceBase::new(), top_index: 0 }
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
    /// The index of the first visible option.
    pub fn top_index(&self) -> i32 { self.top_index }
    /// Set the index of the first visible option.
    pub fn set_top_index(&mut self, value: i32) { self.top_index = value; }
}

impl HasBase for ListBoxField {
    fn base(&self) -> &AnnotationBase { &self.base }
    fn base_mut(&mut self) -> &mut AnnotationBase { &mut self.base }
}
impl HasWidgetBase for ListBoxField {
    fn widget(&self) -> &WidgetBase { &self.widget }
    fn widget_mut(&mut self) -> &mut WidgetBase { &mut self.widget }
}
impl HasFieldBase for ListBoxField {
    fn field(&self) -> &FieldBase { &self.field }
    fn field_mut(&mut self) -> &mut FieldBase { &mut self.field }
    fn pdf_field_type(&self) -> &'static [u8] { b"Ch" }
    fn pdf_field_flags(&self) -> i64 {
        choice_flags(self.choice())
    }
    fn configure(&mut self, request: crate::public::field::FieldConfig<'_>) -> bool {
        use crate::public::choice_field::ChoiceField;
        use crate::public::field::FieldConfig;
        match request {
            FieldConfig::AddOption(item) => {
                self.add_option(item.to_string());
                true
            }
            FieldConfig::RemoveOption(item) => {
                self.delete_option(item);
                true
            }
            _ => false,
        }
    }
    fn pdf_field_entries(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        let mut out = crate::public::choice_field::choice_entries(self.choice());
        // §12.7.4.4: /TI is the option scrolled to the top of the box.
        // Zero is the first option, which is where a box opens anyway.
        if self.top_index > 0 {
            out.push(("TI", Entry::Int(self.top_index as i64)));
        }
        out
    }
}
impl HasChoiceBase for ListBoxField {
    fn choice(&self) -> &ChoiceBase { &self.choice }
    fn choice_mut(&mut self) -> &mut ChoiceBase { &mut self.choice }
}
impl Annotation for ListBoxField {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::Widget }
}
impl Field for ListBoxField {}
impl ChoiceField for ListBoxField {}
