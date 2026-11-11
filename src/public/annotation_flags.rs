//! `Aspose.Pdf.Annotations.AnnotationFlags` — annotation enum, per `capabilities/annotation_flags.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `AnnotationFlags` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum AnnotationFlags {
    /// `Default`.
    #[default]
    Default = 0,
    /// `Invisible`.
    Invisible = 1,
    /// `Hidden`.
    Hidden = 2,
    /// `Print`.
    Print = 4,
    /// `NoZoom`.
    NoZoom = 8,
    /// `NoRotate`.
    NoRotate = 16,
    /// `NoView`.
    NoView = 32,
    /// `ReadOnly`.
    ReadOnly = 64,
    /// `Locked`.
    Locked = 128,
    /// `ToggleNoView`.
    ToggleNoView = 256,
    /// `LockedContents`.
    LockedContents = 512,
}
