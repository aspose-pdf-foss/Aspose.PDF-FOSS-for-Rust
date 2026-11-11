//! `Aspose.Pdf.Forms.CheckboxField` — a check-box form field, per
//! `capabilities/checkbox_field.yaml`.
//!
//! Extends [`Field`]. `Clone` (returns `System.Object`)
//! is dropped — Rust value semantics replace it.

/// The name PDF gives an unchecked box (§12.7.4.2.3).
const OFF_STATE: &str = "Off";
/// The conventional on-state name when a box declares no export value.
const DEFAULT_ON_STATE: &str = "Yes";

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::box_style::BoxStyle;
use crate::public::document::Document;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;

/// A check-box form field.
pub struct CheckboxField {
    base: AnnotationBase,
    widget: WidgetBase,
    field: FieldBase,
    allowed_states: Vec<String>,
    style: BoxStyle,
    export_value: String,
}

impl CheckboxField {
    fn build(base: AnnotationBase) -> Self {
        Self {
            base,
            widget: WidgetBase::new(),
            field: FieldBase::new(),
            allowed_states: Vec::new(),
            style: BoxStyle::Check,
            export_value: String::new(),
        }
    }

    /// Construct within a [`Document`].
    pub fn new(_document: &Document) -> Self {
        Self::build(AnnotationBase::default())
    }

    /// Construct on a [`Page`] at the given rectangle.
    pub fn on_page(page: &Page, rect: Rectangle) -> Self {
        let mut base = AnnotationBase::new(rect);
        base.page_index = page.number() as i32;
        Self::build(base)
    }

    /// Add an on-state export value to the check box.
    pub fn add_option(&mut self, export_value: String) {
        self.allowed_states.push(export_value);
    }

    /// Add an on-state export value with a widget rectangle.
    pub fn add_option_at(&mut self, export_value: String, _rect: Rectangle) {
        self.allowed_states.push(export_value);
    }

    /// Add an on-state export value at a page index with a rectangle.
    pub fn add_option_on_page(&mut self, export_value: String, _page_index: i32, _rect: Rectangle) {
        self.allowed_states.push(export_value);
    }

    /// The allowed on-state names.
    pub fn allowed_states(&self) -> &[String] {
        &self.allowed_states
    }

    /// The check-mark style.
    pub fn style(&self) -> BoxStyle {
        self.style
    }
    /// Set the check-mark style.
    pub fn set_style(&mut self, value: BoxStyle) {
        self.style = value;
    }

    /// Whether the box is checked.
    ///
    /// Derived from the field's value rather than held separately, so
    /// there is one source of truth: PDF represents a checked box as a
    /// `/V` naming its on state and an unchecked one as `/V /Off`
    /// (§12.7.4.2.3). A value set directly through
    /// [`Field::set_value`] is therefore
    /// reflected here, and vice versa.
    pub fn checked(&self) -> bool {
        let v = &self.field().value;
        !v.is_empty() && v != OFF_STATE
    }
    /// Set whether the box is checked. Checking it writes the
    /// [`export_value`](CheckboxField::export_value) as the field's
    /// value, or `Yes` when none was given — the conventional on-state
    /// name; unchecking writes `Off`.
    pub fn set_checked(&mut self, value: bool) {
        let state = if !value {
            OFF_STATE.to_string()
        } else if self.export_value.is_empty() {
            DEFAULT_ON_STATE.to_string()
        } else {
            self.export_value.clone()
        };
        self.field_mut().value = state;
    }

    /// The export value used when the box is checked.
    pub fn export_value(&self) -> &str {
        &self.export_value
    }
    /// Set the export value.
    pub fn set_export_value(&mut self, value: String) {
        self.export_value = value;
    }
}

impl HasBase for CheckboxField {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
}
impl HasWidgetBase for CheckboxField {
    fn widget(&self) -> &WidgetBase {
        &self.widget
    }
    fn widget_mut(&mut self) -> &mut WidgetBase {
        &mut self.widget
    }
}
impl HasFieldBase for CheckboxField {
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
        // §12.5.6.19 Table 189: the mark a viewer draws is the /MK
        // caption, one ZapfDingbats character. Without it the box falls
        // back to the viewer's own default, whatever the style says.
        vec![(
            "MK",
            Entry::Dict(vec![("CA", Entry::Text(self.style.zapf_caption().to_string()))]),
        )]
    }
}
impl Annotation for CheckboxField {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Widget
    }
}
impl Field for CheckboxField {}
