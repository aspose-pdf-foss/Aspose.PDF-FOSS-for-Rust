//! `Aspose.Pdf.Rows` — a table's ordered row collection, per
//! `capabilities/rows.yaml`.
//!
//! The `add*` methods append a row and return a mutable handle to
//! it. The object-identity `Remove` / `IndexOf` overloads and the
//! .NET enumeration / dispose infrastructure are dropped; mutation
//! is by index.

use crate::public::row::Row;

/// An ordered, 0-indexed collection of [`Row`]s.
#[derive(Default)]
pub struct Rows {
    items: Vec<Row>,
}

impl std::fmt::Debug for Rows {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rows").field("count", &self.items.len()).finish()
    }
}

impl Rows {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an empty row, returning a handle to it.
    pub fn add(&mut self) -> &mut Row {
        self.items.push(Row::new());
        self.items.last_mut().expect("just pushed")
    }

    /// Append `row`, returning a handle to it.
    pub fn add_row(&mut self, row: Row) -> &mut Row {
        self.items.push(row);
        self.items.last_mut().expect("just pushed")
    }

    /// Remove the row at a 0-based index.
    pub fn remove_at(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
        }
    }

    /// Remove `count` rows starting at a 0-based index.
    pub fn remove_range(&mut self, index: usize, count: usize) {
        let len = self.items.len();
        if index >= len || count == 0 {
            return;
        }
        let end = index.saturating_add(count).min(len);
        self.items.drain(index..end);
    }

    /// The number of rows.
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// The row at a 0-based index.
    pub fn get(&self, index: usize) -> Option<&Row> {
        self.items.get(index)
    }

    /// The row at a 0-based index, mutably.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Row> {
        self.items.get_mut(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_index() {
        let mut rows = Rows::new();
        rows.add().cells_mut().add_text("a");
        rows.add();
        assert_eq!(rows.count(), 2);
        assert_eq!(rows.get(0).unwrap().cells().count(), 1);
        rows.remove_at(1);
        assert_eq!(rows.count(), 1);
    }
}
