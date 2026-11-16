//! `Aspose.Pdf.Annotations.AnnotationStateModel` — annotation enum, per `capabilities/annotation_state_model.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Which family of review states an annotation's state belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AnnotationStateModel {
    /// No model; nothing is written.
    #[default]
    Undefined = 0,
    /// States that simply flag the annotation.
    Marked = 1,
    /// States that record a review verdict.
    Review = 2,
}
