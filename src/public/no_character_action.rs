//! `Aspose.Pdf.Text.TextEditOptions.NoCharacterAction` — what happens
//! when a replacement font lacks a required character, per
//! `capabilities/no_character_action.yaml`.
//!
//! Variant ordinals match the canonical Aspose.PDF 26.4.0 enum.

/// Behaviour when a replacement font lacks a required character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum NoCharacterAction {
    /// Throw when a required character is missing from the font.
    #[default]
    ThrowException = 0,
    /// Fall back to a standard font for the missing character.
    UseStandardFont = 1,
    /// Replace the text anyway, ignoring the missing character.
    ReplaceAnyway = 2,
    /// Replace the font so it covers the missing character.
    ReplaceFonts = 3,
    /// Use the configured custom replacement font.
    UseCustomReplacementFont = 4,
}
