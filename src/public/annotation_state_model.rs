//! `Aspose.Pdf.Annotations.AnnotationStateModel` — annotation enum, per `capabilities/annotation_state_model.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `AnnotationStateModel` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AnnotationStateModel {
    /// `Undefined`.
    #[default]
    Undefined = 0,
    /// `Marked`.
    Marked = 1,
    /// `Review`.
    Review = 2,
}
