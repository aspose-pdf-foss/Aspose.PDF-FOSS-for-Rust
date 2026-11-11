//! `Aspose.Pdf.Forms.DocMDPAccessPermissions` — forms enum, per `capabilities/doc_mdp_access_permissions.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `DocMDPAccessPermissions` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
#[allow(clippy::upper_case_acronyms)]
pub enum DocMDPAccessPermissions {
    /// `NoChanges`.
    #[default]
    NoChanges = 1,
    /// `FillingInForms`.
    FillingInForms = 2,
    /// `AnnotationModification`.
    AnnotationModification = 3,
}
