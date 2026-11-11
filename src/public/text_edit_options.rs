//! `Aspose.Pdf.Text.TextEditOptions` — options controlling how text is
//! replaced while editing, per `capabilities/text_edit_options.yaml`.
//!
//! Canonically extends the empty `TextOptions` base. The bag carries
//! the font-handling behaviours (missing-character action, font
//! replacement, language transformation, clipping-path processing) and
//! an optional replacement font. It is consulted by the editing
//! constructors of [`TextFragmentAbsorber`](crate::TextFragmentAbsorber).
//!
//! **Carried, not consulted.** They describe how content-stream text
//! replacement would behave, and that capability is not wired — the
//! methods that would use it panic, naming the slug that unblocks them.
//! See the crate README's scope note.

use crate::public::clipping_paths_processing_mode::ClippingPathsProcessingMode;
use crate::public::font::Font;
use crate::public::font_replace::FontReplace;
use crate::public::language_transformation::LanguageTransformation;
use crate::public::no_character_action::NoCharacterAction;

/// Options controlling how text is replaced while editing.
#[derive(Debug, Clone, Default)]
pub struct TextEditOptions {
    replacement_font: Option<Font>,
    no_character_behavior: NoCharacterAction,
    font_replace_behavior: FontReplace,
    allow_language_transformation: bool,
    language_transformation_behavior: LanguageTransformation,
    clipping_paths_processing: ClippingPathsProcessingMode,
    to_attempt_get_underline_from_source: bool,
}

impl TextEditOptions {
    /// Create options with a missing-character behaviour.
    pub fn with_no_character_behavior(no_character_behavior: NoCharacterAction) -> Self {
        Self { no_character_behavior, ..Self::default() }
    }

    /// Create options with a font-replacement behaviour.
    pub fn with_font_replace_behavior(font_replace_behavior: FontReplace) -> Self {
        Self { font_replace_behavior, ..Self::default() }
    }

    /// Create options choosing whether language transformation is allowed.
    pub fn with_allow_language_transformation(allow_language_transformation: bool) -> Self {
        Self { allow_language_transformation, ..Self::default() }
    }

    /// Create options with a language-transformation behaviour.
    pub fn with_language_transformation_behavior(
        language_transformation_behavior: LanguageTransformation,
    ) -> Self {
        Self { language_transformation_behavior, ..Self::default() }
    }

    /// The font used to replace missing glyphs, if set.
    pub fn replacement_font(&self) -> Option<&Font> {
        self.replacement_font.as_ref()
    }

    /// Set the font used to replace missing glyphs.
    pub fn set_replacement_font(&mut self, value: Font) {
        self.replacement_font = Some(value);
    }

    /// The behaviour when a replacement font lacks a required character.
    pub fn no_character_behavior(&self) -> NoCharacterAction {
        self.no_character_behavior
    }

    /// Set the behaviour when a replacement font lacks a required character.
    pub fn set_no_character_behavior(&mut self, value: NoCharacterAction) {
        self.no_character_behavior = value;
    }

    /// The font-replacement behaviour.
    pub fn font_replace_behavior(&self) -> FontReplace {
        self.font_replace_behavior
    }

    /// Set the font-replacement behaviour.
    pub fn set_font_replace_behavior(&mut self, value: FontReplace) {
        self.font_replace_behavior = value;
    }

    /// Whether language transformation is allowed.
    pub fn allow_language_transformation(&self) -> bool {
        self.allow_language_transformation
    }

    /// Set whether language transformation is allowed.
    pub fn set_allow_language_transformation(&mut self, value: bool) {
        self.allow_language_transformation = value;
    }

    /// The language-transformation behaviour.
    pub fn language_transformation_behavior(&self) -> LanguageTransformation {
        self.language_transformation_behavior
    }

    /// Set the language-transformation behaviour.
    pub fn set_language_transformation_behavior(&mut self, value: LanguageTransformation) {
        self.language_transformation_behavior = value;
    }

    /// How existing clipping paths are processed.
    pub fn clipping_paths_processing(&self) -> ClippingPathsProcessingMode {
        self.clipping_paths_processing
    }

    /// Set how existing clipping paths are processed.
    pub fn set_clipping_paths_processing(&mut self, value: ClippingPathsProcessingMode) {
        self.clipping_paths_processing = value;
    }

    /// Whether to attempt to carry the underline over from the source.
    pub fn to_attempt_get_underline_from_source(&self) -> bool {
        self.to_attempt_get_underline_from_source
    }

    /// Set whether to attempt to carry the underline over from the source.
    pub fn set_to_attempt_get_underline_from_source(&mut self, value: bool) {
        self.to_attempt_get_underline_from_source = value;
    }
}
