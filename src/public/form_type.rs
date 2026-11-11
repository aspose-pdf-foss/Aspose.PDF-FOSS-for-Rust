//! `Aspose.Pdf.Forms.FormType` — forms enum, per `capabilities/form_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `FormType` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FormType {
    /// `Standard`.
    #[default]
    Standard = 0,
    /// `Static`.
    Static = 1,
    /// `Dynamic`.
    Dynamic = 2,
}
