//! `Aspose.Pdf.Annotations.AnnotationFlags` — annotation enum, per `capabilities/annotation_flags.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Display and interaction flags carried by an annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AnnotationFlags {
    /// No flags set.
    #[default]
    Default = 0,
    /// Hide an annotation whose type the viewer cannot handle, rather than drawing a fallback.
    Invisible = 1,
    /// Neither display nor print the annotation.
    Hidden = 2,
    /// Print the annotation; without this flag it is shown on screen only.
    Print = 4,
    /// Keep the annotation at a fixed size as the page is zoomed.
    NoZoom = 8,
    /// Keep the annotation upright as the page is rotated.
    NoRotate = 16,
    /// Print the annotation but do not show it on screen.
    NoView = 32,
    /// Display the annotation but ignore interaction with it.
    ReadOnly = 64,
    /// Forbid deleting the annotation or changing its properties.
    Locked = 128,
    /// Invert the `NoView` behaviour while the annotation is selected or hovered.
    ToggleNoView = 256,
    /// Forbid changing the annotation's contents, its other properties staying editable.
    LockedContents = 512,
}
