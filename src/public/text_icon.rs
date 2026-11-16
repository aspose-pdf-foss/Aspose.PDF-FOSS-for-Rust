//! `Aspose.Pdf.Annotations.TextIcon` — annotation enum, per `capabilities/text_icon.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// The icon a text annotation is shown with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum TextIcon {
    /// A note-page icon.
    #[default]
    Note = 0,
    /// A comment balloon.
    Comment = 1,
    /// A key icon.
    Key = 2,
    /// A question-mark icon.
    Help = 3,
    /// A mark for a new paragraph.
    NewParagraph = 4,
    /// A pilcrow.
    Paragraph = 5,
    /// An insertion caret.
    Insert = 6,
    /// A tick.
    Check = 7,
    /// A cross.
    Cross = 8,
    /// A circle.
    Circle = 9,
    /// A star.
    Star = 10,
}
