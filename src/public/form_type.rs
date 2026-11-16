//! `Aspose.Pdf.Forms.FormType` — forms enum, per `capabilities/form_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Which form technology a document's form uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FormType {
    /// A plain AcroForm.
    #[default]
    Standard = 0,
    /// A form whose layout is fixed.
    Static = 1,
    /// A form whose layout is built as it is filled.
    Dynamic = 2,
}
