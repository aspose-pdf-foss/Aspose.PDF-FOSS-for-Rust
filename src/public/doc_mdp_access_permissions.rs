//! `Aspose.Pdf.Forms.DocMDPAccessPermissions` — forms enum, per `capabilities/doc_mdp_access_permissions.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// What a reader may still change in a document certified by a DocMDP signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
#[allow(clippy::upper_case_acronyms)]
pub enum DocMDPAccessPermissions {
    /// No change is permitted; any modification breaks the certification.
    #[default]
    NoChanges = 1,
    /// Filling in form fields and signing are permitted.
    FillingInForms = 2,
    /// Filling in form fields, signing and annotating are permitted.
    AnnotationModification = 3,
}
