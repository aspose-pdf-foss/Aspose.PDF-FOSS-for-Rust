//! `Aspose.Pdf.Forms.ComboBoxField` — a combo-box choice field,
//! per `capabilities/combo_box_field.yaml`.

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

/// A combo-box (drop-down) choice field.
pub struct ComboBoxField {
    base: AnnotationBase,
    widget: WidgetBase,
    field: FieldBase,
    choice: ChoiceBase,
    editable: bool,
    spell_check: bool,
}

impl ComboBoxField {
    fn build(base: AnnotationBase) -> Self {
        Self { base, widget: WidgetBase::new(), field: FieldBase::new(),
               choice: ChoiceBase::new(), editable: false, spell_check: true }
    }
    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self { Self::build(AnnotationBase::default()) }
    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self::build(base)
    }
    /// Whether the combo box accepts custom text.
    pub fn editable(&self) -> bool { self.editable }
    /// Set whether the combo box accepts custom text.
    pub fn set_editable(&mut self, value: bool) { self.editable = value; }
    /// Whether spell-checking is enabled. `true` by default, which is
    /// the format's own default: the `/Ff` flag is DoNotSpellCheck, and
    /// a field carrying no such bit is checked. Table 230 allows the
    /// flag only on an editable combo box, so it is written only when
    /// [`editable`](ComboBoxField::editable) is set.
    pub fn spell_check(&self) -> bool { self.spell_check }
    /// Set whether spell-checking is enabled.
    pub fn set_spell_check(&mut self, value: bool) { self.spell_check = value; }
}

impl HasBase for ComboBoxField {
    fn base(&self) -> &AnnotationBase { &self.base }
    fn base_mut(&mut self) -> &mut AnnotationBase { &mut self.base }
}
impl HasWidgetBase for ComboBoxField {
    fn widget(&self) -> &WidgetBase { &self.widget }
    fn widget_mut(&mut self) -> &mut WidgetBase { &mut self.widget }
}
impl HasFieldBase for ComboBoxField {
    fn field(&self) -> &FieldBase { &self.field }
    fn field_mut(&mut self) -> &mut FieldBase { &mut self.field }
    fn pdf_field_type(&self) -> &'static [u8] { b"Ch" }
    fn pdf_field_flags(&self) -> i64 {
        // Table 230: Combo 18 makes it a drop-down at all, Edit 19
        // lets the user type, DoNotSpellCheck 23 is a negation written
        // for the `false` state, and MultiSelect 22 and
        // CommitOnSelChange 27 come from the shared choice base.
        let mut f = 1 << 17;
        if self.editable {
            f |= 1 << 18;
            // DoNotSpellCheck 23. Table 230 states the flag may not be
            // used unless Combo and Edit are both set, and Combo always
            // is here, so an uneditable combo box writes nothing: there
            // is no text for a viewer to check.
            if !self.spell_check {
                f |= 1 << 22;
            }
        }
        f | choice_flags(self.choice())
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
        crate::public::choice_field::choice_entries(self.choice())
    }
}
impl HasChoiceBase for ComboBoxField {
    fn choice(&self) -> &ChoiceBase { &self.choice }
    fn choice_mut(&mut self) -> &mut ChoiceBase { &mut self.choice }
}
impl Annotation for ComboBoxField {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::Widget }
}
impl Field for ComboBoxField {}
impl ChoiceField for ComboBoxField {}
