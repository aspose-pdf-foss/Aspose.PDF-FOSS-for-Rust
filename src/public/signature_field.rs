//! `Aspose.Pdf.Forms.SignatureField` — a signature form field, per
//! `capabilities/signature_field.yaml`.
//!
//! Extends [`Field`]. The streaming `Sign(Signature,
//! Stream, String)` overload, the `ExtractImage` / `ExtractCertificate`
//! helpers (Stream / X509 / ImageFormat), and `ExtractCertificateObject`
//! are dropped — actual signing is deferred to the encryption beat.

use crate::public::annotation::sealed::HasBase;
use crate::public::annotation::{Annotation, AnnotationBase};
use crate::public::annotation_type::AnnotationType;
use crate::public::document::Document;
use crate::public::field::sealed::HasFieldBase;
use crate::public::field::{Field, FieldBase};
use crate::public::page::Page;
use crate::public::rectangle::Rectangle;
use crate::public::signature::Signature;
use crate::public::widget_annotation::sealed::HasWidgetBase;
use crate::public::widget_annotation::WidgetBase;

/// A signature form field.
pub struct SignatureField {
    base: AnnotationBase,
    widget: WidgetBase,
    field: FieldBase,
    signature: std::option::Option<Signature>,
}

impl SignatureField {
    fn build(base: AnnotationBase) -> Self {
        Self { base, widget: WidgetBase::new(), field: FieldBase::new(), signature: None }
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
    /// Attach a signature to the field. The cryptographic signing is
    /// deferred (the signature is recorded for the field surface).
    pub fn sign(&mut self, signature: Signature) {
        self.signature = Some(signature);
    }
    /// The attached signature, if any.
    pub fn signature(&self) -> std::option::Option<&Signature> {
        self.signature.as_ref()
    }
}

impl HasBase for SignatureField {
    fn base(&self) -> &AnnotationBase { &self.base }
    fn base_mut(&mut self) -> &mut AnnotationBase { &mut self.base }
}
impl HasWidgetBase for SignatureField {
    fn widget(&self) -> &WidgetBase { &self.widget }
    fn widget_mut(&mut self) -> &mut WidgetBase { &mut self.widget }
}
impl HasFieldBase for SignatureField {
    fn field(&self) -> &FieldBase { &self.field }
    fn field_mut(&mut self) -> &mut FieldBase { &mut self.field }
    fn pdf_field_type(&self) -> &'static [u8] { b"Sig" }
}
impl Annotation for SignatureField {
    fn annotation_type(&self) -> AnnotationType { AnnotationType::Widget }
}
impl Field for SignatureField {}
