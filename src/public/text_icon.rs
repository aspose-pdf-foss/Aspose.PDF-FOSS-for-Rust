//! `Aspose.Pdf.Annotations.TextIcon` — annotation enum, per `capabilities/text_icon.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// `TextIcon` — see the canonical Aspose.PDF documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum TextIcon {
    /// `Note`.
    #[default]
    Note = 0,
    /// `Comment`.
    Comment = 1,
    /// `Key`.
    Key = 2,
    /// `Help`.
    Help = 3,
    /// `NewParagraph`.
    NewParagraph = 4,
    /// `Paragraph`.
    Paragraph = 5,
    /// `Insert`.
    Insert = 6,
    /// `Check`.
    Check = 7,
    /// `Cross`.
    Cross = 8,
    /// `Circle`.
    Circle = 9,
    /// `Star`.
    Star = 10,
}
