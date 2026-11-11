//! `Aspose.Pdf.Annotations.ReplyType` — annotation enum, per `capabilities/reply_type.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `ReplyType` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ReplyType {
    /// `Undefined`.
    #[default]
    Undefined = 0,
    /// `Reply`.
    Reply = 1,
    /// `Group`.
    Group = 2,
}
