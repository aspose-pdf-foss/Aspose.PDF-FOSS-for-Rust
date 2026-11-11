//! `Aspose.Pdf.Row` — a table row, per `capabilities/row.yaml`.
//!
//! Holds the row's [`Cells`] plus border / background / default cell
//! styling and the row-height / break layout. The `ICloneable.Clone`
//! member is dropped.

use crate::public::border_info::BorderInfo;
use crate::public::cells::Cells;
use crate::public::color::Color;
use crate::public::margin_info::MarginInfo;
use crate::public::text_state::TextState;
use crate::public::vertical_alignment::VerticalAlignment;

/// A table row.
#[derive(Debug)]
pub struct Row {
    background_color: Color,
    border: BorderInfo,
    cells: Cells,
    default_cell_border: BorderInfo,
    min_row_height: f64,
    fixed_row_height: f64,
    is_in_new_page: bool,
    is_row_broken: bool,
    default_cell_text_state: TextState,
    default_cell_padding: MarginInfo,
    vertical_alignment: VerticalAlignment,
}

impl Default for Row {
    fn default() -> Self {
        Self {
            background_color: Color::empty(),
            border: BorderInfo::new(),
            cells: Cells::new(),
            default_cell_border: BorderInfo::new(),
            min_row_height: 0.0,
            fixed_row_height: 0.0,
            is_in_new_page: false,
            is_row_broken: false,
            default_cell_text_state: TextState::default(),
            default_cell_padding: MarginInfo::new(),
            vertical_alignment: VerticalAlignment::None,
        }
    }
}

impl Row {
    /// Construct an empty row.
    pub fn new() -> Self {
        Self::default()
    }

    /// The row background colour, filled behind the whole row before
    /// its cells are drawn. An empty colour is no fill.
    pub fn background_color(&self) -> Color {
        self.background_color
    }
    /// Set the row background colour.
    pub fn set_background_color(&mut self, value: Color) {
        self.background_color = value;
    }

    /// The row border.
    pub fn border(&self) -> &BorderInfo {
        &self.border
    }
    /// Set the row border.
    pub fn set_border(&mut self, value: BorderInfo) {
        self.border = value;
    }

    /// The row's cells.
    pub fn cells(&self) -> &Cells {
        &self.cells
    }
    /// The row's cells, mutably.
    pub fn cells_mut(&mut self) -> &mut Cells {
        &mut self.cells
    }

    /// The default border applied to the row's cells: the width a cell
    /// of this row is stroked with where it sets none of its own, with
    /// the table's own default behind it.
    pub fn default_cell_border(&self) -> &BorderInfo {
        &self.default_cell_border
    }
    /// Set the default border applied to the row's cells.
    pub fn set_default_cell_border(&mut self, value: BorderInfo) {
        self.default_cell_border = value;
    }

    /// The minimum row height, in points.
    pub fn min_row_height(&self) -> f64 {
        self.min_row_height
    }
    /// Set the minimum row height, in points.
    pub fn set_min_row_height(&mut self, value: f64) {
        self.min_row_height = value;
    }

    /// The fixed row height, in points (0 = auto).
    pub fn fixed_row_height(&self) -> f64 {
        self.fixed_row_height
    }
    /// Set the fixed row height, in points.
    pub fn set_fixed_row_height(&mut self, value: f64) {
        self.fixed_row_height = value;
    }

    /// Whether the row starts on a new page.
    pub fn is_in_new_page(&self) -> bool {
        self.is_in_new_page
    }
    /// Set whether the row starts on a new page.
    pub fn set_is_in_new_page(&mut self, value: bool) {
        self.is_in_new_page = value;
    }

    /// Whether the row may break across pages.
    /// **Carried, not applied.** Page assignment moves whole rows: a
    /// row that does not fit is placed anyway and overflows, rather
    /// than being split across the page boundary this flag would
    /// permit.
    pub fn is_row_broken(&self) -> bool {
        self.is_row_broken
    }
    /// Set whether the row may break across pages.
    pub fn set_is_row_broken(&mut self, value: bool) {
        self.is_row_broken = value;
    }

    /// The default text state for the row's cells: the font, size,
    /// colour and spacing a cell of this row renders with where neither
    /// the text fragment nor the cell set one, with the table's own
    /// default behind it.
    pub fn default_cell_text_state(&self) -> &TextState {
        &self.default_cell_text_state
    }
    /// Set the default text state for the row's cells.
    pub fn set_default_cell_text_state(&mut self, value: TextState) {
        self.default_cell_text_state = value;
    }

    /// The default padding applied to the row's cells.
    ///
    /// Carried, not applied, and blocked one level lower: a cell's own
    /// padding ([`Cell::margin`](crate::Cell::margin)) is unapplied
    /// too — the renderer insets a cell's text by a fixed amount — so
    /// there is nothing for a row default to override.
    pub fn default_cell_padding(&self) -> MarginInfo {
        self.default_cell_padding
    }
    /// Set the default padding applied to the row's cells.
    pub fn set_default_cell_padding(&mut self, value: MarginInfo) {
        self.default_cell_padding = value;
    }

    /// The vertical alignment of the row's cells.
    pub fn vertical_alignment(&self) -> VerticalAlignment {
        self.vertical_alignment
    }
    /// Set the vertical alignment of the row's cells.
    pub fn set_vertical_alignment(&mut self, value: VerticalAlignment) {
        self.vertical_alignment = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_and_properties() {
        let mut r = Row::new();
        r.cells_mut().add_text("x");
        r.cells_mut().add_text("y");
        assert_eq!(r.cells().count(), 2);
        r.set_min_row_height(20.0);
        r.set_is_in_new_page(true);
        assert_eq!(r.min_row_height(), 20.0);
        assert!(r.is_in_new_page());
    }
}
