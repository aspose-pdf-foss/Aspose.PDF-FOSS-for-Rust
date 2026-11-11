//! `Aspose.Pdf.Cell` — a single table cell, per
//! `capabilities/cell.yaml`.
//!
//! Holds the cell's content [`Paragraphs`], its border / margin /
//! background / alignment styling, the default text state, and the
//! span / width layout. The `BackgroundImage` (`Aspose.Pdf.Image`,
//! not yet shipped) and `ICloneable.Clone` members are dropped.

use crate::public::border_info::BorderInfo;
use crate::public::color::Color;
use crate::public::horizontal_alignment::HorizontalAlignment;
use crate::public::margin_info::MarginInfo;
use crate::public::paragraphs::Paragraphs;
use crate::public::rectangle::Rectangle;
use crate::public::text_state::TextState;
use crate::public::vertical_alignment::VerticalAlignment;

/// A table cell.
#[derive(Debug)]
pub struct Cell {
    is_no_border: bool,
    margin: MarginInfo,
    border: BorderInfo,
    background_color: Color,
    background_image_file: String,
    alignment: HorizontalAlignment,
    default_cell_text_state: TextState,
    is_override_by_fragment: bool,
    paragraphs: Paragraphs,
    is_word_wrapped: bool,
    vertical_alignment: VerticalAlignment,
    col_span: i32,
    row_span: i32,
    width: f64,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            is_no_border: false,
            margin: MarginInfo::new(),
            border: BorderInfo::new(),
            background_color: Color::empty(),
            background_image_file: String::new(),
            alignment: HorizontalAlignment::None,
            default_cell_text_state: TextState::default(),
            is_override_by_fragment: false,
            paragraphs: Paragraphs::new(),
            is_word_wrapped: true,
            vertical_alignment: VerticalAlignment::None,
            col_span: 1,
            row_span: 1,
            width: 0.0,
        }
    }
}

impl Cell {
    /// Construct an empty cell.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a cell with an explicit width from `rectangle`.
    pub fn with_rectangle(rectangle: Rectangle) -> Self {
        Self { width: rectangle.width(), ..Self::default() }
    }

    /// Whether the cell suppresses its border.
    pub fn is_no_border(&self) -> bool {
        self.is_no_border
    }
    /// Set whether the cell suppresses its border.
    pub fn set_is_no_border(&mut self, value: bool) {
        self.is_no_border = value;
    }

    /// The cell's content padding.
    pub fn margin(&self) -> MarginInfo {
        self.margin
    }
    /// Set the cell's content padding.
    pub fn set_margin(&mut self, value: MarginInfo) {
        self.margin = value;
    }

    /// The cell border.
    pub fn border(&self) -> &BorderInfo {
        &self.border
    }
    /// Set the cell border.
    pub fn set_border(&mut self, value: BorderInfo) {
        self.border = value;
    }

    /// The cell background colour.
    pub fn background_color(&self) -> Color {
        self.background_color
    }
    /// Set the cell background colour.
    pub fn set_background_color(&mut self, value: Color) {
        self.background_color = value;
    }

    /// The background image file path.
    ///
    /// **Carried, not applied,** and not for the reason this note used
    /// to give — it pointed at cell line breaking, which has nothing to
    /// do with a background image. The real one: drawing it means
    /// adding an image XObject to the page's resources, and a
    /// paragraph's render declares content bytes and font resources
    /// only, without knowing its page. The cell's
    /// [`background_color`](Cell::background_color) IS painted, because
    /// a colour needs no resource.
    pub fn background_image_file(&self) -> &str {
        &self.background_image_file
    }
    /// Set the background image file path.
    pub fn set_background_image_file(&mut self, value: impl Into<String>) {
        self.background_image_file = value.into();
    }

    /// The horizontal alignment of the cell's content, applied at render
    /// when that content's width can be measured (text can be).
    pub fn alignment(&self) -> HorizontalAlignment {
        self.alignment
    }
    /// Set the horizontal content alignment.
    pub fn set_alignment(&mut self, value: HorizontalAlignment) {
        self.alignment = value;
    }

    /// The default text state for the cell's content: its font and size
    /// style the text the cell holds, except where a fragment sets its
    /// own or the cell defers via
    /// [`is_override_by_fragment`](Self::is_override_by_fragment). The
    /// rest of the state is carried but not yet emitted.
    pub fn default_cell_text_state(&self) -> &TextState {
        &self.default_cell_text_state
    }
    /// Set the default text state for the cell's content.
    pub fn set_default_cell_text_state(&mut self, value: TextState) {
        self.default_cell_text_state = value;
    }

    /// Whether per-fragment state overrides the cell default. When set,
    /// the cell's [`default_cell_text_state`](Self::default_cell_text_state)
    /// is not applied to the text it holds.
    pub fn is_override_by_fragment(&self) -> bool {
        self.is_override_by_fragment
    }
    /// Set whether per-fragment state overrides the cell default.
    pub fn set_is_override_by_fragment(&mut self, value: bool) {
        self.is_override_by_fragment = value;
    }

    /// The font, size and colour a fragment renders with inside this
    /// cell: its own where it sets them, and `default` where it does
    /// not — the cell's `DefaultCellTextState` with its row's and its
    /// table's already resolved behind it. A cell marked
    /// [`is_override_by_fragment`](Self::is_override_by_fragment) defers
    /// to the fragment entirely. A size of zero means neither set one
    /// and the render path's own default applies.
    ///
    /// Font, size, fill colour and the `Tc` / `Tw` spacing are what the
    /// render path reads; the rest of a text state (the stroking colour,
    /// the decoration flags) is carried but not emitted yet.
    pub(crate) fn resolved_text_style(
        &self,
        fragment: &crate::public::text_fragment::TextFragment,
        default: &TextState,
    ) -> (Option<crate::public::font::Font>, f32, TextState) {
        let own = &fragment.text_state().state;
        if self.is_override_by_fragment {
            return (own.font().cloned(), own.font_size(), own.clone());
        }
        let font = own.font().or_else(|| default.font()).cloned();
        let size = if own.font_size() > 0.0 {
            own.font_size()
        } else {
            default.font_size()
        };
        // The state the run is drawn with: the fragment's own where it
        // set something, this cell's default where it did not. A fully
        // transparent colour and zero spacing are the unset states.
        let mut state = own.clone();
        if own.foreground_color().a() <= 0.0 {
            state.set_foreground_color(default.foreground_color());
        }
        if own.character_spacing() == 0.0 {
            state.set_character_spacing(default.character_spacing());
        }
        if own.word_spacing() == 0.0 {
            state.set_word_spacing(default.word_spacing());
        }
        (font, size, state)
    }

    /// The cell's content paragraphs.
    pub fn paragraphs(&self) -> &Paragraphs {
        &self.paragraphs
    }
    /// The cell's content paragraphs, mutably.
    pub fn paragraphs_mut(&mut self) -> &mut Paragraphs {
        &mut self.paragraphs
    }
    /// Replace the cell's content paragraphs.
    pub fn set_paragraphs(&mut self, value: Paragraphs) {
        self.paragraphs = value;
    }

    /// Whether the cell wraps long text.
    /// **Carried, not applied.** A cell's paragraphs are placed on one
    /// line each; there is no cell-level line breaking for this flag to
    /// switch off, and adding one would relayout every existing table.
    /// [`Cell::row_span`] and [`Cell::background_image_file`] are
    /// carried the same way — the first wants a row-occupancy map the
    /// table layout does not build, the second an image type this
    /// subset does not ship.
    pub fn is_word_wrapped(&self) -> bool {
        self.is_word_wrapped
    }
    /// Set whether the cell wraps long text.
    pub fn set_is_word_wrapped(&mut self, value: bool) {
        self.is_word_wrapped = value;
    }

    /// How the cell's content sits in its row, applied at render:
    /// `Top` (and the unset default) hangs it from the row's top
    /// edge, `Center` centres it, `Bottom` rests it on the row's
    /// bottom edge.
    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }
    /// Set the vertical content alignment.
    pub fn set_vertical_alignment(&mut self, value: VerticalAlignment) {
        self.vertical_alignment = value;
    }

    /// The number of columns the cell spans.
    pub fn col_span(&self) -> i32 {
        self.col_span
    }
    /// Set the number of columns the cell spans.
    pub fn set_col_span(&mut self, value: i32) {
        self.col_span = value;
    }

    /// The number of rows the cell spans. Carried on the cell; the
    /// table's render does not merge rows, so every cell occupies the
    /// row it was added to.
    /// **Carried, not applied** — see [`Cell::is_word_wrapped`].
    /// [`Cell::col_span`] IS applied; spanning rows needs the occupancy
    /// map that spanning columns does not.
    pub fn row_span(&self) -> i32 {
        self.row_span
    }
    /// Set the number of rows the cell spans.
    pub fn set_row_span(&mut self, value: i32) {
        self.row_span = value;
    }

    /// The cell width, in points.
    pub fn width(&self) -> f64 {
        self.width
    }
    /// Set the cell width, in points.
    pub fn set_width(&mut self, value: f64) {
        self.width = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_properties() {
        let mut c = Cell::new();
        assert_eq!(c.col_span(), 1);
        assert_eq!(c.row_span(), 1);
        assert!(c.is_word_wrapped());
        assert_eq!(c.paragraphs().count(), 0);
        c.set_col_span(2);
        c.set_width(50.0);
        c.set_background_color(Color::from_rgb(0.0, 1.0, 0.0));
        assert_eq!(c.col_span(), 2);
        assert_eq!(c.width(), 50.0);
        assert_eq!(c.background_color().a(), 1.0);
    }
}
