//! `Aspose.Pdf.Forms.ButtonField` — a push-button form field, per
//! `capabilities/button_field.yaml`.
//!
//! Extends [`Field`]. The icon properties
//! (`Aspose.Pdf.XForm`) and the `AddImage` (`System.Drawing.Image`)
//! helper are dropped (those types are not in the shipped subset); the
//! [`icon_fit`](ButtonField::icon_fit) accessor ships, paired with
//! [`icon_fit_mut`](ButtonField::icon_fit_mut) so the options can be
//! set — the canonical member is a getter returning an object a caller
//! mutates through.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::document::Document;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::icon_caption_position::IconCaptionPosition;
use crate::public::icon_fit::IconFit;
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;

/// A push-button form field.
pub struct ButtonField {
    base: AnnotationBase,
    widget: WidgetBase,
    field: FieldBase,
    normal_caption: String,
    rollover_caption: String,
    alternate_caption: String,
    ic_position: IconCaptionPosition,
    icon_fit: IconFit,
}

impl ButtonField {
    fn build(base: AnnotationBase) -> Self {
        Self {
            base,
            widget: WidgetBase::new(),
            field: FieldBase::new(),
            normal_caption: String::new(),
            rollover_caption: String::new(),
            alternate_caption: String::new(),
            ic_position: IconCaptionPosition::NoIcon,
            icon_fit: IconFit::new(),
        }
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

    /// The caption shown in the normal state.
    pub fn normal_caption(&self) -> &str {
        &self.normal_caption
    }
    /// Set the normal-state caption.
    pub fn set_normal_caption(&mut self, value: String) {
        self.normal_caption = value;
    }

    /// The caption shown on rollover.
    pub fn rollover_caption(&self) -> &str {
        &self.rollover_caption
    }
    /// Set the rollover caption.
    pub fn set_rollover_caption(&mut self, value: String) {
        self.rollover_caption = value;
    }

    /// The alternate (down-state) caption.
    pub fn alternate_caption(&self) -> &str {
        &self.alternate_caption
    }
    /// Set the alternate caption.
    pub fn set_alternate_caption(&mut self, value: String) {
        self.alternate_caption = value;
    }

    /// The icon / caption relative position.
    pub fn ic_position(&self) -> IconCaptionPosition {
        self.ic_position
    }
    /// Set the icon / caption relative position.
    pub fn set_ic_position(&mut self, value: IconCaptionPosition) {
        self.ic_position = value;
    }

    /// The icon-scaling options for the button's image.
    pub fn icon_fit(&self) -> &IconFit {
        &self.icon_fit
    }
    /// The icon-scaling options, mutably. The canonical member is a
    /// getter returning an object a caller mutates through; Rust needs
    /// a second accessor to offer the same reach, the way
    /// [`Document::form`](crate::Document::form) is paired with
    /// `form_mut`.
    pub fn icon_fit_mut(&mut self) -> &mut IconFit {
        &mut self.icon_fit
    }
}

impl HasBase for ButtonField {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
}
impl HasWidgetBase for ButtonField {
    fn widget(&self) -> &WidgetBase {
        &self.widget
    }
    fn widget_mut(&mut self) -> &mut WidgetBase {
        &mut self.widget
    }
}
impl HasFieldBase for ButtonField {
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
        // Table 229: Pushbutton is bit 17 — a button that holds no
        // value, as opposed to a checkbox or a radio group.
        1 << 16
    }
    fn configure(&mut self, request: crate::public::field::FieldConfig<'_>) -> bool {
        use crate::public::field::FieldConfig;
        match request {
            FieldConfig::Caption(text) => {
                self.normal_caption = text.to_string();
                true
            }
            _ => false,
        }
    }
    fn pdf_field_entries(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // §12.5.6.19 Table 189: the captions a push button shows live
        // in its /MK appearance-characteristics dictionary — /CA in the
        // normal state, /RC under the pointer, /AC while pressed — and
        // /TP says how caption and icon are laid out. A button with no
        // caption and the default layout needs no /MK at all.
        let mut mk: Vec<(&'static str, Entry)> = Vec::new();
        if !self.normal_caption.is_empty() {
            mk.push(("CA", Entry::Text(self.normal_caption.clone())));
        }
        if !self.rollover_caption.is_empty() {
            mk.push(("RC", Entry::Text(self.rollover_caption.clone())));
        }
        if !self.alternate_caption.is_empty() {
            mk.push(("AC", Entry::Text(self.alternate_caption.clone())));
        }
        if self.ic_position != IconCaptionPosition::NoIcon {
            mk.push(("TP", Entry::Int(self.ic_position as i64)));
        }
        if mk.is_empty() {
            Vec::new()
        } else {
            vec![("MK", Entry::Dict(mk))]
        }
    }
}
impl Annotation for ButtonField {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Widget
    }
}
impl Field for ButtonField {}
