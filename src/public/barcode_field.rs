//! `Aspose.Pdf.Forms.BarcodeField` — a barcode text-box form field,
//! per `capabilities/barcode_field.yaml`.
//!
//! Extends [`TextBoxField`] (Rust models the inheritance by embedding a
//! base `TextBoxField` + [`Deref`], so the inherited text-box surface —
//! `multiline`, `max_len`, `scrollable` and the rest — applies here too).

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::document::Document;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::symbology::Symbology;
use crate::public::text_box_field::TextBoxField;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;
use std::ops::{Deref, DerefMut};

/// A barcode text-box form field. The barcode-geometry properties
/// are read-only (populated by the engine when the barcode is
/// generated).
pub struct BarcodeField {
    base: TextBoxField,
    resolution: i32,
    caption: String,
    symbology: Symbology,
    x_sym_width: i32,
    x_sym_height: i32,
    ecc: i32,
}

impl BarcodeField {
    fn build(base: AnnotationBase) -> Self {
        Self { base: TextBoxField::build(base),
               resolution: 0, caption: String::new(), symbology: Symbology::PDF417,
               x_sym_width: 0, x_sym_height: 0, ecc: 0 }
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
    /// The barcode resolution, in DPI.
    pub fn resolution(&self) -> i32 { self.resolution }
    /// The human-readable caption.
    pub fn caption(&self) -> &str { &self.caption }
    /// The barcode symbology.
    pub fn symbology(&self) -> Symbology { self.symbology }
    /// The module (symbol) width.
    pub fn x_sym_width(&self) -> i32 { self.x_sym_width }
    /// The module (symbol) height.
    pub fn x_sym_height(&self) -> i32 { self.x_sym_height }
    /// The error-correction level.
    pub fn ecc(&self) -> i32 { self.ecc }
}

impl Deref for BarcodeField {
    type Target = TextBoxField;
    fn deref(&self) -> &TextBoxField { &self.base }
}
impl DerefMut for BarcodeField {
    fn deref_mut(&mut self) -> &mut TextBoxField { &mut self.base }
}

// The inherited bases live in the embedded TextBoxField, so the field
// and its base read and write the same state.
impl HasBase for BarcodeField {
    fn base(&self) -> &AnnotationBase { self.base.base() }
    fn base_mut(&mut self) -> &mut AnnotationBase { self.base.base_mut() }
}
impl HasWidgetBase for BarcodeField {
    fn widget(&self) -> &WidgetBase { self.base.widget() }
    fn widget_mut(&mut self) -> &mut WidgetBase { self.base.widget_mut() }
}
impl HasFieldBase for BarcodeField {
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
impl Annotation for BarcodeField {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::Widget }
}
impl Field for BarcodeField {}
