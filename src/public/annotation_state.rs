//! `Aspose.Pdf.Annotations.AnnotationState` — annotation enum, per `capabilities/annotation_state.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The review state a markup annotation has been marked with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AnnotationState {
    /// No state; nothing is written.
    #[default]
    Undefined = 0,
    /// Flagged by the reviewer.
    Marked = 1,
    /// Explicitly not flagged.
    Unmarked = 2,
    /// The change was accepted.
    Accepted = 3,
    /// The change was rejected.
    Rejected = 4,
    /// The review was cancelled.
    Cancelled = 5,
    /// The review is finished.
    Completed = 6,
    /// Reviewed with no verdict recorded.
    None = 7,
}
