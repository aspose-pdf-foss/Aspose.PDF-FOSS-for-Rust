//! `Aspose.Pdf.Annotations.Measure.NumberFormatList` — the ordered
//! list of `NumberFormat` entries for one measurement axis, per
//! `capabilities/number_format_list.yaml`.
//!
//! The `Measure`-taking constructor is dropped (see
//! `targets/rust/drops.yaml`); construct via `Default` and populate
//! with [`NumberFormatList::add`].

use crate::public::number_format::NumberFormat;

/// An ordered list of [`NumberFormat`] entries.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NumberFormatList {
    items: Vec<NumberFormat>,
}

impl NumberFormatList {
    /// Append a format entry (canonical `Add`).
    pub fn add(&mut self, value: NumberFormat) {
        self.items.push(value);
    }

    /// Insert a format entry at `index` (canonical `Insert`).
    pub fn insert(&mut self, index: usize, value: NumberFormat) {
        self.items.insert(index, value);
    }

    /// Remove the entry at `index` (canonical `RemoveAt`).
    pub fn remove_at(&mut self, index: usize) {
        self.items.remove(index);
    }

    /// The entry at `index`, or `None` if out of range (canonical
    /// indexer).
    pub fn at(&self, index: usize) -> Option<&NumberFormat> {
        self.items.get(index)
    }

    /// The number of entries (canonical `Count`).
    pub fn count(&self) -> usize {
        self.items.len()
    }
}
