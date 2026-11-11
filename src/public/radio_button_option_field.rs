//! `Aspose.Pdf.Forms.RadioButtonOptionField` — one option of a
//! radio-button group, per
//! `capabilities/radio_button_option_field.yaml`.
//!
//! Extends [`Field`]. `Caption` is the option's caption
//! [`TextFragment`].

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::box_style::BoxStyle;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::text_fragment::TextFragment;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;

/// One selectable option within a radio-button group.
pub struct RadioButtonOptionField {
    base: AnnotationBase,
    widget: WidgetBase,
    field: FieldBase,
    option_name: String,
    style: BoxStyle,
    caption: TextFragment,
}

impl RadioButtonOptionField {
    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self {
            base,
            widget: WidgetBase::new(),
            field: FieldBase::new(),
            option_name: String::new(),
            style: BoxStyle::Circle,
            caption: TextFragment::new(),
        }
    }

    /// The on-state name of this option.
    pub fn option_name(&self) -> &str {
        &self.option_name
    }
    /// Set the on-state name.
    pub fn set_option_name(&mut self, value: String) {
        self.option_name = value;
    }

    /// The button style.
    pub fn style(&self) -> BoxStyle {
        self.style
    }
    /// Set the button style.
    pub fn set_style(&mut self, value: BoxStyle) {
        self.style = value;
    }

    /// The option's caption text fragment.
    ///
    /// Carried, not written: this is the label drawn beside the button,
    /// and a widget is written as a dictionary — a label is either page
    /// content or an appearance stream, and neither is what a field
    /// contributes. The *mark* inside the button is
    /// [`style`](RadioButtonOptionField::style), which is written.
    pub fn caption(&self) -> &TextFragment {
        &self.caption
    }
    /// Set the option's caption text fragment.
    pub fn set_caption(&mut self, value: TextFragment) {
        self.caption = value;
    }
}

impl HasBase for RadioButtonOptionField {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
}
impl HasWidgetBase for RadioButtonOptionField {
    fn widget(&self) -> &WidgetBase {
        &self.widget
    }
    fn widget_mut(&mut self) -> &mut WidgetBase {
        &mut self.widget
    }
}
impl HasFieldBase for RadioButtonOptionField {
    fn field(&self) -> &FieldBase {
        &self.field
    }
    fn field_mut(&mut self) -> &mut FieldBase {
        &mut self.field
    }
    fn pdf_field_type(&self) -> &'static [u8] {
        b"Btn"
    }
    fn pdf_field_entries(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.19 Table 189: the mark a viewer draws inside the
        // button is the /MK caption, one ZapfDingbats character. The
        // group writes its own; an option added to a form in its own
        // right is a widget of its own and needs it too, or it falls
        // back to whatever the viewer defaults to.
        vec![(
            "MK",
            Entry::Dict(vec![("CA", Entry::Text(self.style.zapf_caption().to_string()))]),
        )]
    }
}
impl Annotation for RadioButtonOptionField {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Widget
    }
}
impl Field for RadioButtonOptionField {}
