//! `Aspose.Pdf.Facades.PropertyFlag` — a settable field property flag,
//! per `capabilities/property_flag.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0
//! `Aspose.Pdf.Facades.PropertyFlag` enum.

/// A form-field property flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum PropertyFlag {
    /// The field is read-only.
    #[default]
    ReadOnly = 0,
    /// The field is required.
    Required = 1,
    /// The field is excluded from export.
    NoExport = 2,
    /// Invalid flag.
    InvalidFlag = 3,
}
