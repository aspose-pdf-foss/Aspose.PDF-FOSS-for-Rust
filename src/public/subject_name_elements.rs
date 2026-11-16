//! `Aspose.Pdf.Forms.SubjectNameElements` — forms enum, per `capabilities/subject_name_elements.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// A component of a signing certificate's subject name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SubjectNameElements {
    /// Common name.
    #[default]
    CN = 0,
    /// Organisation.
    O = 1,
    /// Locality.
    L = 2,
    /// Organisational unit.
    OU = 3,
    /// State or province.
    S = 4,
    /// Country.
    C = 5,
    /// Email address.
    E = 6,
}
