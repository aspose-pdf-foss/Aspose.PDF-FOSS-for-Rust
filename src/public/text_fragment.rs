//! `Aspose.Pdf.Text.TextFragment` — a run of text with a shared
//! styling state, per `capabilities/text_fragment.yaml`.
//!
//! A strict subset: the text write-back + `rectangle`, the
//! font-mutation `text_state`, and the positioned-insertion surface
//! (`text` constructor + `position`) used by
//! [`TextBuilder`](crate::TextBuilder). The wider segment /
//! alignment / hyperlink / note surface (and the `BaseParagraph`
//! base) is deferred.
//!
//! A freshly constructed fragment is unplaced — its rectangle is
//! trivial until a text absorber locates it in a document or a text
//! builder positions it. Canonically `TextState` is a get-only
//! property that returns a *mutable* state; Rust splits that into
//! [`text_state`](TextFragment::text_state) /
//! [`text_state_mut`](TextFragment::text_state_mut).

use crate::public::base_paragraph::{impl_base_paragraph, ParagraphBase};
use crate::public::position::Position;
use crate::public::rectangle::Rectangle;
use crate::public::text_fragment_state::TextFragmentState;

/// A run of text sharing one styling state.
///
/// Canonically a [`BaseParagraph`](crate::BaseParagraph) — the
/// `base` field carries that shared layout geometry, which (holding
/// a `Box<dyn Hyperlink>`) keeps `TextFragment` from deriving
/// `Clone` / `PartialEq`.
#[derive(Debug)]
pub struct TextFragment {
    base: ParagraphBase,
    text: String,
    text_state: TextFragmentState,
    position: Position,
    rectangle: Rectangle,
}

impl TextFragment {
    /// Construct an empty text fragment.
    pub fn new() -> Self {
        Self {
            base: ParagraphBase::default(),
            text: String::new(),
            text_state: TextFragmentState::new(),
            position: Position::default(),
            rectangle: Rectangle::trivial(),
        }
    }

    /// Construct a text fragment carrying `text`.
    pub fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::new()
        }
    }

    /// Construct a located fragment from a text absorber: its text,
    /// insertion position, bounding rectangle, and font size.
    /// Crate-internal — fragments are located, not user-built.
    pub(crate) fn located(
        text: String,
        position: Position,
        rectangle: Rectangle,
        font_size: f32,
    ) -> Self {
        let mut text_state = TextFragmentState::new();
        text_state.set_font_size(font_size);
        Self {
            base: ParagraphBase::default(),
            text,
            text_state,
            position,
            rectangle,
        }
    }

    /// The fragment's text.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Set the fragment's text.
    pub fn set_text(&mut self, value: impl Into<String>) {
        self.text = value.into();
    }

    /// The fragment's bounding rectangle. Trivial until the
    /// fragment is located (by an absorber) or positioned.
    pub fn rectangle(&self) -> Rectangle {
        self.rectangle
    }

    /// The fragment's styling state.
    pub fn text_state(&self) -> &TextFragmentState {
        &self.text_state
    }
    /// The fragment's styling state, mutably (the canonical
    /// `TextState` property returns a mutable state).
    pub fn text_state_mut(&mut self) -> &mut TextFragmentState {
        &mut self.text_state
    }

    /// The fragment's insertion position.
    pub fn position(&self) -> Position {
        self.position
    }
    /// Set the fragment's insertion position.
    pub fn set_position(&mut self, value: Position) {
        self.position = value;
    }
}

impl Default for TextFragment {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::public::base_paragraph::sealed::Renderable for TextFragment {
    /// Render this fragment at its [`position`](TextFragment::position)
    /// — a text-showing run with its font / size — when added to a
    /// page's paragraph collection.
    fn as_text_fragment(&self) -> Option<&TextFragment> {
        Some(self)
    }

    fn rendered_width(&self) -> Option<f64> {
        if self.text().is_empty() {
            return None;
        }
        let state = self.text_state();
        let size = state.font_size() as f64;
        let size = if size > 0.0 { size } else { 12.0 };
        let base_font = state
            .font()
            .map(|f| f.base_font().to_string())
            .unwrap_or_else(|| "Helvetica".to_string());
        Some(crate::public::text_builder::text_width(&base_font, self.text(), size))
    }

    fn render_content(
        &self,
        _page_height: f64,
    ) -> Option<crate::public::base_paragraph::sealed::RenderedParagraph> {
        crate::public::text_builder::render_text_fragment(self).map(|(content, font)| {
            crate::public::base_paragraph::sealed::RenderedParagraph {
                content,
                fonts: vec![font],
                continuations: Vec::new(),
            }
        })
    }
}
impl_base_paragraph!(TextFragment);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::font_repository::FontRepository;

    #[test]
    fn construct_and_edit() {
        let mut f = TextFragment::with_text("Hello");
        assert_eq!(f.text(), "Hello");
        f.set_text("World");
        assert_eq!(f.text(), "World");
        assert!(f.rectangle().is_trivial());
    }

    #[test]
    fn position_and_mutable_state() {
        let mut f = TextFragment::new();
        f.set_position(Position::new(100.0, 500.0));
        assert_eq!(f.position(), Position::new(100.0, 500.0));

        f.text_state_mut().set_font(FontRepository::find_font("Helvetica"));
        f.text_state_mut().set_font_size(14.0);
        assert_eq!(f.text_state().font().unwrap().font_name(), "Helvetica");
        assert_eq!(f.text_state().font_size(), 14.0);
    }
}
