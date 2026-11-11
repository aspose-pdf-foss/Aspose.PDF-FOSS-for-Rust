//! `Aspose.Pdf.Forms.RadioButtonField` — a radio-button group field,
//! per `capabilities/radio_button_field.yaml`.
//!
//! Extends [`ChoiceField`].

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::box_style::BoxStyle;
use crate::public::choice_field::sealed::HasChoiceBase;
use crate::public::choice_field::{ChoiceBase, ChoiceField};
use crate::public::document::Document;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::option::Option as FormOption;
use crate::public::page::Page;
use crate::public::radio_button_option_field::RadioButtonOptionField;
use crate::public::rectangle::Rectangle;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;

/// A radio-button group field.
pub struct RadioButtonField {
    base: AnnotationBase,
    widget: WidgetBase,
    field: FieldBase,
    choice: ChoiceBase,
    style: BoxStyle,
    no_toggle_to_off: bool,
}

impl RadioButtonField {
    fn build(base: AnnotationBase) -> Self {
        Self {
            base,
            widget: WidgetBase::new(),
            field: FieldBase::new(),
            choice: ChoiceBase::new(),
            style: BoxStyle::Circle,
            no_toggle_to_off: false,
        }
    }

    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self {
        Self::build(AnnotationBase::default())
    }

    /// Construct on a [`Page`].
    pub fn from_page(page: &Page) -> Self {
        let base = AnnotationBase {
            page_index: page.number() as i32,
            ..AnnotationBase::default()
        };
        Self::build(base)
    }

    /// Add a pre-built radio option to the group.
    pub fn add(&mut self, option: RadioButtonOptionField) {
        let mut o = FormOption::new();
        o.set_value(option.option_name().to_string());
        o.set_name(option.option_name().to_string());
        self.choice.options.add(o);
    }

    /// Add an option by export value with a widget rectangle.
    pub fn add_option_at(&mut self, export_value: String, _rect: Rectangle) {
        self.add_option(export_value);
    }

    /// The button style.
    pub fn style(&self) -> BoxStyle {
        self.style
    }
    /// Set the button style.
    pub fn set_style(&mut self, value: BoxStyle) {
        self.style = value;
    }

    /// Whether the selected button cannot be toggled off.
    pub fn no_toggle_to_off(&self) -> bool {
        self.no_toggle_to_off
    }
    /// Set whether the selected button cannot be toggled off.
    pub fn set_no_toggle_to_off(&mut self, value: bool) {
        self.no_toggle_to_off = value;
    }
}

impl HasBase for RadioButtonField {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
}
impl HasWidgetBase for RadioButtonField {
    fn widget(&self) -> &WidgetBase {
        &self.widget
    }
    fn widget_mut(&mut self) -> &mut WidgetBase {
        &mut self.widget
    }
}
impl HasFieldBase for RadioButtonField {
    fn field(&self) -> &FieldBase {
        &self.field
    }
    fn field_mut(&mut self) -> &mut FieldBase {
        &mut self.field
    }
    fn pdf_field_type(&self) -> &'static [u8] {
        b"Btn"
    }
    fn pdf_field_flags(&self) -> i64 {
        // Table 229: Radio is bit 16. Without it a viewer treats the
        // field as a checkbox and the buttons stop being exclusive.
        // NoToggleToOff is bit 15 — with it the selected button cannot
        // be clicked off, leaving the group always answered.
        let mut f = 1 << 15;
        if self.no_toggle_to_off {
            f |= 1 << 14;
        }
        f
    }
    fn pdf_field_entries(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.19 Table 189: the mark a viewer draws is the /MK
        // caption, one ZapfDingbats character. Without it the box falls
        // back to the viewer's own default, whatever the style says.
        vec![(
            "MK",
            Entry::Dict(vec![("CA", Entry::Text(self.style.zapf_caption().to_string()))]),
        )]
    }

}
impl HasChoiceBase for RadioButtonField {
    fn choice(&self) -> &ChoiceBase {
        &self.choice
    }
    fn choice_mut(&mut self) -> &mut ChoiceBase {
        &mut self.choice
    }
}
impl Annotation for RadioButtonField {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Widget
    }
}
impl Field for RadioButtonField {}
impl ChoiceField for RadioButtonField {}
