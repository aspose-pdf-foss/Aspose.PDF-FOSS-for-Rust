//! `Aspose.Pdf.Annotations.AnnotationState` — annotation enum, per `capabilities/annotation_state.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `AnnotationState` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AnnotationState {
    /// `Undefined`.
    #[default]
    Undefined = 0,
    /// `Marked`.
    Marked = 1,
    /// `Unmarked`.
    Unmarked = 2,
    /// `Accepted`.
    Accepted = 3,
    /// `Rejected`.
    Rejected = 4,
    /// `Cancelled`.
    Cancelled = 5,
    /// `Completed`.
    Completed = 6,
    /// `None`.
    None = 7,
}
