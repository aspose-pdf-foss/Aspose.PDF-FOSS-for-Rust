//! `Aspose.Pdf.Text.TextParagraph` — a word-wrapped, aligned block
//! of text, per `capabilities/text_paragraph.yaml`.
//!
//! Accumulates lines (via the `append_line*` methods) and carries
//! the layout properties (alignment, indents, position / wrap box,
//! margin) that a [`TextBuilder`](crate::TextBuilder) consumes when
//! it lays the paragraph into a page. Both the string and
//! `TextFragment`-taking `AppendLine` overloads ship (with optional
//! per-line `TextState` and line spacing).

use crate::public::horizontal_alignment::HorizontalAlignment;
use crate::public::margin_info::MarginInfo;
use crate::public::position::Position;
use crate::public::rectangle::Rectangle;
use crate::public::text_fragment::TextFragment;
use crate::public::text_state::TextState;
use crate::public::vertical_alignment::VerticalAlignment;

/// One appended line: its text plus optional per-line styling and
/// spacing. Crate-internal — these fields are consumed by
/// `TextBuilder` when it lays the paragraph into a page (that
/// consumer lands with the text-builder beat).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct ParagraphLine {
    pub text: String,
    pub state: Option<TextState>,
    pub line_spacing: Option<f32>,
}

/// A multi-line, aligned text block.
#[derive(Debug, Clone)]
pub struct TextParagraph {
    pub(crate) lines: Vec<ParagraphLine>,
    horizontal_alignment: HorizontalAlignment,
    vertical_alignment: VerticalAlignment,
    first_line_indent: f32,
    subsequent_lines_indent: f32,
    position: Position,
    rectangle: Rectangle,
    margin: MarginInfo,
}

impl Default for TextParagraph {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            horizontal_alignment: HorizontalAlignment::None,
            vertical_alignment: VerticalAlignment::None,
            first_line_indent: 0.0,
            subsequent_lines_indent: 0.0,
            position: Position::default(),
            rectangle: Rectangle::trivial(),
            margin: MarginInfo::new(),
        }
    }
}

impl TextParagraph {
    /// Construct an empty paragraph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a line of text.
    pub fn append_line(&mut self, line: impl Into<String>) {
        self.lines.push(ParagraphLine {
            text: line.into(),
            state: None,
            line_spacing: None,
        });
    }

    /// Append a line of text with a custom line spacing (points).
    pub fn append_line_spaced(&mut self, line: impl Into<String>, line_spacing: f32) {
        self.lines.push(ParagraphLine {
            text: line.into(),
            state: None,
            line_spacing: Some(line_spacing),
        });
    }

    /// Append a line of text with a per-line text state.
    pub fn append_line_with_state(
        &mut self,
        line: impl Into<String>,
        text_state: TextState,
    ) {
        self.lines.push(ParagraphLine {
            text: line.into(),
            state: Some(text_state),
            line_spacing: None,
        });
    }

    /// Append a line of text with a per-line text state and spacing.
    pub fn append_line_with_state_spaced(
        &mut self,
        line: impl Into<String>,
        text_state: TextState,
        line_spacing: f32,
    ) {
        self.lines.push(ParagraphLine {
            text: line.into(),
            state: Some(text_state),
            line_spacing: Some(line_spacing),
        });
    }

    /// Append a line taken from a [`TextFragment`]; its text and its
    /// font / size carry over as the line's state.
    pub fn append_line_fragment(&mut self, fragment: &TextFragment) {
        let mut state = TextState::default();
        if let Some(font) = fragment.text_state().font() {
            state.set_font(font.clone());
        }
        state.set_font_size(fragment.text_state().font_size());
        self.lines.push(ParagraphLine {
            text: fragment.text().to_string(),
            state: Some(state),
            line_spacing: None,
        });
    }

    /// Append a line taken from a [`TextFragment`] with an explicit
    /// per-line text state.
    pub fn append_line_fragment_with_state(
        &mut self,
        fragment: &TextFragment,
        text_state: TextState,
    ) {
        self.lines.push(ParagraphLine {
            text: fragment.text().to_string(),
            state: Some(text_state),
            line_spacing: None,
        });
    }

    /// Append a line taken from a [`TextFragment`] with an explicit
    /// per-line text state and line spacing.
    pub fn append_line_fragment_with_state_spaced(
        &mut self,
        fragment: &TextFragment,
        text_state: TextState,
        line_spacing: f32,
    ) {
        self.lines.push(ParagraphLine {
            text: fragment.text().to_string(),
            state: Some(text_state),
            line_spacing: Some(line_spacing),
        });
    }

    /// The horizontal alignment within the box.
    pub fn horizontal_alignment(&self) -> HorizontalAlignment {
        self.horizontal_alignment
    }
    /// Set the horizontal alignment within the box.
    pub fn set_horizontal_alignment(&mut self, value: HorizontalAlignment) {
        self.horizontal_alignment = value;
    }

    /// The vertical alignment within the box.
    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }
    /// Set the vertical alignment within the box.
    pub fn set_vertical_alignment(&mut self, value: VerticalAlignment) {
        self.vertical_alignment = value;
    }

    /// The first-line left indent, in points.
    pub fn first_line_indent(&self) -> f32 {
        self.first_line_indent
    }
    /// Set the first-line left indent, in points.
    pub fn set_first_line_indent(&mut self, value: f32) {
        self.first_line_indent = value;
    }

    /// The indent applied to lines after the first, in points.
    pub fn subsequent_lines_indent(&self) -> f32 {
        self.subsequent_lines_indent
    }
    /// Set the indent applied to lines after the first, in points.
    pub fn set_subsequent_lines_indent(&mut self, value: f32) {
        self.subsequent_lines_indent = value;
    }

    /// The origin used when no wrap rectangle is set.
    pub fn position(&self) -> Position {
        self.position
    }
    /// Set the origin used when no wrap rectangle is set.
    pub fn set_position(&mut self, value: Position) {
        self.position = value;
    }

    /// The box the paragraph lays out in: its width is what
    /// [`TextBuilder::append_paragraph`](crate::TextBuilder::append_paragraph)
    /// wraps against, and its top-left corner is where the first line
    /// starts. Lines are not clipped to its height — a paragraph taller
    /// than the box runs past the bottom of it.
    pub fn rectangle(&self) -> Rectangle {
        self.rectangle
    }
    /// Set the layout box. See [`rectangle`](Self::rectangle).
    pub fn set_rectangle(&mut self, value: Rectangle) {
        self.rectangle = value;
    }

    /// The margins applied to the box before layout.
    pub fn margin(&self) -> MarginInfo {
        self.margin
    }
    /// Set the margins applied to the box before layout.
    pub fn set_margin(&mut self, value: MarginInfo) {
        self.margin = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_properties() {
        let mut p = TextParagraph::new();
        assert_eq!(p.horizontal_alignment(), HorizontalAlignment::None);
        assert_eq!(p.vertical_alignment(), VerticalAlignment::None);
        assert_eq!(p.first_line_indent(), 0.0);
        assert!(p.rectangle().is_trivial());

        p.set_horizontal_alignment(HorizontalAlignment::Left);
        p.set_rectangle(Rectangle::new(72.0, 500.0, 300.0, 720.0, true));
        p.set_margin(MarginInfo::with_margins(5.0, 5.0, 5.0, 5.0));
        assert_eq!(p.horizontal_alignment(), HorizontalAlignment::Left);
        assert_eq!(p.rectangle().width(), 228.0);
        assert_eq!(p.margin().left(), 5.0);
    }

    #[test]
    fn append_line_overloads_store_lines() {
        let mut p = TextParagraph::new();
        p.append_line("first");
        p.append_line_spaced("second", 14.0);
        let mut st = TextState::default();
        st.set_font_size(11.0);
        p.append_line_with_state("third", st.clone());
        p.append_line_with_state_spaced("fourth", st, 16.0);
        assert_eq!(p.lines.len(), 4);
        assert_eq!(p.lines[0].text, "first");
        assert_eq!(p.lines[1].line_spacing, Some(14.0));
        assert!(p.lines[2].state.is_some());
        assert_eq!(p.lines[3].line_spacing, Some(16.0));
    }

    #[test]
    fn append_line_fragment_overloads() {
        let mut p = TextParagraph::new();

        let mut frag = TextFragment::with_text("from-fragment");
        frag.text_state_mut().set_font_size(13.0);
        p.append_line_fragment(&frag);

        let mut st = TextState::default();
        st.set_font_size(9.0);
        p.append_line_fragment_with_state(&frag, st.clone());
        p.append_line_fragment_with_state_spaced(&frag, st, 15.0);

        assert_eq!(p.lines.len(), 3);
        assert_eq!(p.lines[0].text, "from-fragment");
        // the bare overload carries the fragment's own font size
        assert_eq!(p.lines[0].state.as_ref().unwrap().font_size(), 13.0);
        // the explicit-state overloads use the supplied state
        assert_eq!(p.lines[1].state.as_ref().unwrap().font_size(), 9.0);
        assert_eq!(p.lines[2].line_spacing, Some(15.0));
    }
}
