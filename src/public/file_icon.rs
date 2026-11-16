//! `Aspose.Pdf.Annotations.FileIcon` — annotation enum, per `capabilities/file_icon.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The icon a file attachment annotation is shown with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum FileIcon {
    /// A drawing-pin icon.
    #[default]
    PushPin = 0,
    /// A graph icon.
    Graph = 1,
    /// A paperclip icon.
    Paperclip = 2,
    /// A luggage-tag icon.
    Tag = 3,
}
