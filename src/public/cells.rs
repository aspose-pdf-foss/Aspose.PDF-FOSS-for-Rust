//! `Aspose.Pdf.Cells` — a row's ordered cell collection, per
//! `capabilities/cells.yaml`.
//!
//! The `add*` methods append a cell and return a mutable handle to
//! it. The object-identity `Remove` overloads and the .NET
//! enumeration / dispose infrastructure are dropped; mutation is by
//! index.

use crate::public::cell::Cell;
use crate::public::text_fragment::TextFragment;
use crate::public::text_state::TextState;

/// An ordered, 0-indexed collection of [`Cell`]s.
#[derive(Default)]
pub struct Cells {
    items: Vec<Cell>,
}

impl std::fmt::Debug for Cells {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cells").field("count", &self.items.len()).finish()
    }
}

impl Cells {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an empty cell, returning a handle to it.
    pub fn add(&mut self) -> &mut Cell {
        self.items.push(Cell::new());
        self.items.last_mut().expect("just pushed")
    }

    /// Append `cell`, returning a handle to it.
    pub fn add_cell(&mut self, cell: Cell) -> &mut Cell {
        self.items.push(cell);
        self.items.last_mut().expect("just pushed")
    }

    /// Append a cell containing `text`, returning a handle to it.
    pub fn add_text(&mut self, text: impl Into<String>) -> &mut Cell {
        let mut cell = Cell::new();
        cell.paragraphs_mut().add(Box::new(TextFragment::with_text(text)));
        self.add_cell(cell)
    }

    /// Append a cell containing `text`, styled by `state` through the
    /// cell's `DefaultCellTextState` — the font and size it carries are
    /// what the text renders in.
    pub fn add_text_with_state(
        &mut self,
        text: impl Into<String>,
        state: TextState,
    ) -> &mut Cell {
        let mut cell = Cell::new();
        cell.set_default_cell_text_state(state);
        cell.paragraphs_mut().add(Box::new(TextFragment::with_text(text)));
        self.add_cell(cell)
    }

    /// Append a cell containing `fragment`, returning a handle to it.
    pub fn add_fragment(&mut self, fragment: TextFragment) -> &mut Cell {
        let mut cell = Cell::new();
        cell.paragraphs_mut().add(Box::new(fragment));
        self.add_cell(cell)
    }

    /// Insert a cell at a 0-based index (clamped to the end).
    pub fn insert(&mut self, index: usize, cell: Cell) {
        let at = index.min(self.items.len());
        self.items.insert(at, cell);
    }

    /// Remove `count` cells starting at a 0-based index.
    pub fn remove_range(&mut self, index: usize, count: usize) {
        let len = self.items.len();
        if index >= len || count == 0 {
            return;
        }
        let end = index.saturating_add(count).min(len);
        self.items.drain(index..end);
    }

    /// The number of cells.
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// The cell at a 0-based index.
    pub fn get(&self, index: usize) -> Option<&Cell> {
        self.items.get(index)
    }

    /// The cell at a 0-based index, mutably.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Cell> {
        self.items.get_mut(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_text_and_index() {
        let mut cells = Cells::new();
        cells.add_text("a");
        cells.add_text("b");
        cells.add();
        assert_eq!(cells.count(), 3);
        assert_eq!(cells.get(0).unwrap().paragraphs().count(), 1);
        cells.remove_range(2, 1);
        assert_eq!(cells.count(), 2);
    }
}
