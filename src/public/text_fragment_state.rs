//! `Aspose.Pdf.Text.TextFragmentState` — the per-fragment text
//! state returned by [`TextFragment`](crate::TextFragment), per
//! `capabilities/text_fragment_state.yaml`.
//!
//! Canonically derives from [`TextState`]. Rust has no inheritance, so
//! the fragment state embeds a `TextState` and hands it out through
//! [`Deref`], which is how the inherited surface — colours, spacing,
//! alignment, the decoration flags — reaches a fragment's state. `font`
//! and `font_size` are declared here as well because the canonical type
//! overrides them; the rest comes from the base. Setting any of them
//! mutates the fragment's state in place.

use crate::public::font::Font;
use crate::public::text_state::TextState;
use std::ops::{Deref, DerefMut};

/// The text state of a single [`TextFragment`](crate::TextFragment).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextFragmentState {
    /// The base state. Crate-internal as a field — callers reach it
    /// through [`Deref`], the save pipeline reads it directly.
    pub(crate) state: TextState,
}

impl TextFragmentState {
    /// Construct a default fragment state.
    pub fn new() -> Self {
        Self::default()
    }

    /// The font, or `None` when no font has been assigned.
    pub fn font(&self) -> Option<&Font> {
        self.state.font()
    }
    /// Set the font.
    pub fn set_font(&mut self, value: Font) {
        self.state.set_font(value);
    }

    /// The font size, in points.
    pub fn font_size(&self) -> f32 {
        self.state.font_size()
    }
    /// Set the font size, in points.
    pub fn set_font_size(&mut self, value: f32) {
        self.state.set_font_size(value);
    }
}

impl Deref for TextFragmentState {
    type Target = TextState;
    fn deref(&self) -> &TextState {
        &self.state
    }
}

impl DerefMut for TextFragmentState {
    fn deref_mut(&mut self) -> &mut TextState {
        &mut self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::font_repository::FontRepository;

    #[test]
    fn font_and_size_roundtrip() {
        let mut s = TextFragmentState::new();
        assert!(s.font().is_none());
        s.set_font(FontRepository::find_font("Helvetica"));
        s.set_font_size(14.0);
        assert_eq!(s.font().unwrap().font_name(), "Helvetica");
        assert_eq!(s.font_size(), 14.0);
    }
}
