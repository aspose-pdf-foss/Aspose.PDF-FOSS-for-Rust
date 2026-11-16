//! `Aspose.Pdf.Annotations.ReplyType` — annotation enum, per `capabilities/reply_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Whether a markup annotation replies to another or is grouped with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ReplyType {
    /// No reply relationship; nothing is written.
    #[default]
    Undefined = 0,
    /// A reply to the annotation it refers to.
    Reply = 1,
    /// Grouped with the annotation it refers to, sharing its properties.
    Group = 2,
}
