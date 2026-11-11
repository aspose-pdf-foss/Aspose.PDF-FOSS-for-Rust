//! `Aspose.Pdf.Annotations.FileIcon` — annotation enum, per `capabilities/file_icon.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `FileIcon` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FileIcon {
    /// `PushPin`.
    #[default]
    PushPin = 0,
    /// `Graph`.
    Graph = 1,
    /// `Paperclip`.
    Paperclip = 2,
    /// `Tag`.
    Tag = 3,
}
