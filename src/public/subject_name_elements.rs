//! `Aspose.Pdf.Forms.SubjectNameElements` — forms enum, per `capabilities/subject_name_elements.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `SubjectNameElements` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum SubjectNameElements {
    /// `CN`.
    #[default]
    CN = 0,
    /// `O`.
    O = 1,
    /// `L`.
    L = 2,
    /// `OU`.
    OU = 3,
    /// `S`.
    S = 4,
    /// `C`.
    C = 5,
    /// `E`.
    E = 6,
}
