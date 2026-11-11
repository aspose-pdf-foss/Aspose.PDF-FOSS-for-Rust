//! `Aspose.Pdf.Forms.TextBoxField` — a text-entry form field, per
//! `capabilities/text_box_field.yaml`.
//!
//! Extends [`Field`]. It is the base of the
//! barcode / date / number / password / rich-text / file-select
//! field family (F-5). The `AddImage` (`System.Drawing.Image`)
//! helper and the parameterless / array-rectangle / document-
//! rectangle constructors are dropped (see `targets/rust/drops.yaml`).

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::document::Document;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::vertical_alignment::VerticalAlignment;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;
use crate::internal::pending::PENDING;

/// A text-entry form field.
pub struct TextBoxField {
    base: AnnotationBase,
    widget: WidgetBase,
    field: FieldBase,
    multiline: bool,
    spell_check: bool,
    scrollable: bool,
    force_combs: bool,
    max_len: i32,
    text_vertical_alignment: VerticalAlignment,
}

impl TextBoxField {
    /// Build over an annotation base. Crate-internal: the field types
    /// that extend `TextBoxField` construct their inherited half through
    /// it (PDF's `BarcodeField`, `NumberField` and friends all derive
    /// from this class).
    pub(crate) fn build(base: AnnotationBase) -> Self {
        Self {
            base,
            widget: WidgetBase::new(),
            field: FieldBase::new(),
            multiline: false,
            // §12.7.4.2 Table 228: the flag is DoNotSpellCheck, so a
            // field that says nothing is spell-checked. `scrollable`
            // below reads its own default the same way.
            spell_check: true,
            scrollable: true,
            force_combs: false,
            max_len: 0,
            text_vertical_alignment: VerticalAlignment::None,
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

    /// Add a barcode to the field.
    ///
    /// Not implemented — pending `barcode_rendering`. Calling this
    /// panics.
    pub fn add_barcode(&mut self, _data: String) {
        PENDING("barcode_rendering")
    }

    /// Whether the field accepts multiple lines.
    pub fn multiline(&self) -> bool {
        self.multiline
    }
    /// Set whether the field accepts multiple lines.
    pub fn set_multiline(&mut self, value: bool) {
        self.multiline = value;
    }

    /// Whether spell-checking is enabled. `true` by default, which is
    /// the format's own default: the `/Ff` flag is DoNotSpellCheck, and
    /// a field carrying no such bit is checked.
    pub fn spell_check(&self) -> bool {
        self.spell_check
    }
    /// Set whether spell-checking is enabled.
    pub fn set_spell_check(&mut self, value: bool) {
        self.spell_check = value;
    }

    /// Whether the field scrolls long text.
    pub fn scrollable(&self) -> bool {
        self.scrollable
    }
    /// Set whether the field scrolls long text.
    pub fn set_scrollable(&mut self, value: bool) {
        self.scrollable = value;
    }

    /// Whether the field uses comb formatting.
    pub fn force_combs(&self) -> bool {
        self.force_combs
    }
    /// Set whether the field uses comb formatting.
    pub fn set_force_combs(&mut self, value: bool) {
        self.force_combs = value;
    }

    /// The maximum number of characters (`0` = unlimited).
    pub fn max_len(&self) -> i32 {
        self.max_len
    }
    /// Set the maximum number of characters.
    pub fn set_max_len(&mut self, value: i32) {
        self.max_len = value;
    }

    /// The vertical alignment of the field text.
    pub fn text_vertical_alignment(&self) -> VerticalAlignment {
        self.text_vertical_alignment
    }
    /// Set the vertical alignment of the field text.
    pub fn set_text_vertical_alignment(&mut self, value: VerticalAlignment) {
        self.text_vertical_alignment = value;
    }
}

impl HasBase for TextBoxField {
    fn base(&self) -> &AnnotationBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut AnnotationBase {
        &mut self.base
    }
}
impl HasWidgetBase for TextBoxField {
    fn widget(&self) -> &WidgetBase {
        &self.widget
    }
    fn widget_mut(&mut self) -> &mut WidgetBase {
        &mut self.widget
    }
}
impl HasFieldBase for TextBoxField {
    fn field(&self) -> &FieldBase {
        &self.field
    }
    fn field_mut(&mut self) -> &mut FieldBase {
        &mut self.field
    }
    fn pdf_field_flags(&self) -> i64 {
        // Table 228: Multiline 13, DoNotSpellCheck 23, DoNotScroll 24,
        // Comb 25. The last three are negations — the bit says what the
        // field may NOT do — so each is written for the `false` state.
        let mut f = 0;
        if self.multiline {
            f |= 1 << 12;
        }
        if !self.scrollable {
            f |= 1 << 23;
        }
        if self.force_combs {
            f |= 1 << 24;
        }
        if !self.spell_check {
            f |= 1 << 22;
        }
        f
    }

    fn configure(&mut self, request: crate::public::field::FieldConfig<'_>) -> bool {
        use crate::public::field::FieldConfig;
        match request {
            FieldConfig::MaxLen(limit) => {
                self.max_len = limit;
                true
            }
            // §12.7.4.3: comb formatting divides the field into /MaxLen
            // evenly spaced cells, so the cell count IS the limit —
            // asking for comb without one leaves nothing to divide.
            FieldConfig::Comb(cells) => {
                self.max_len = cells;
                self.force_combs = cells > 0;
                true
            }
            FieldConfig::Multiline => {
                self.multiline = true;
                true
            }
            _ => false,
        }
    }
    fn pdf_field_entries(
        &self,
    ) -> Vec<(&'static str, crate::internal::annotation_io::Entry)> {
        use crate::internal::annotation_io::Entry;
        // A comb field is meaningless without the cell count /MaxLen
        // gives it, and a length limit is the caller's instruction
        // either way.
        if self.max_len > 0 {
            vec![("MaxLen", Entry::Int(self.max_len as i64))]
        } else {
            Vec::new()
        }
    }
}
impl Annotation for TextBoxField {
    fn annotation_type(&self) -> AnnotationType {
        AnnotationType::Widget
    }
}
impl Field for TextBoxField {}
