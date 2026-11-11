//! `Aspose.Pdf.PageLabelCollection` — the document's page-label
//! ranges, per `capabilities/page_label_collection.yaml`.
//!
//! Each entry keys a [`PageLabel`] by the 0-based index of the
//! first page it applies to; the label stays in effect until the
//! next range begins. `get_label` performs this floor lookup.

use crate::public::numbering_style::NumberingStyle;
use crate::public::page_label::PageLabel;
use std::collections::BTreeMap;

/// A collection of page-label ranges keyed by start page index.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageLabelCollection {
    ranges: BTreeMap<i32, PageLabel>,
    /// Set when mutated, so `Document::save` knows to (re)write the
    /// catalog's `/PageLabels` number tree.
    dirty: bool,
}

impl PageLabelCollection {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set (or replace) the label that begins at `page_index`.
    pub fn update_label(&mut self, page_index: i32, page_label: PageLabel) {
        self.ranges.insert(page_index, page_label);
        self.dirty = true;
    }

    /// Whether the collection has been mutated since load.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// `(start_index, numbering_style, starting_value, prefix)` tuples,
    /// for save-through serialisation, in ascending index order.
    pub(crate) fn entries_for_save(&self) -> Vec<(i32, NumberingStyle, i32, String)> {
        self.ranges
            .iter()
            .map(|(idx, l)| (*idx, l.numbering_style(), l.starting_value(), l.prefix().to_string()))
            .collect()
    }

    /// Insert a label loaded from the PDF (does not mark the collection
    /// dirty, so a verbatim resave keeps the original).
    pub(crate) fn update_loaded(&mut self, page_index: i32, page_label: PageLabel) {
        self.ranges.insert(page_index, page_label);
    }

    /// The label in effect at `page_index` — the range whose start
    /// is the greatest index not after `page_index`. Returns a
    /// default [`PageLabel`] when no range covers the index.
    pub fn get_label(&self, page_index: i32) -> PageLabel {
        self.ranges
            .range(..=page_index)
            .next_back()
            .map(|(_, label)| label.clone())
            .unwrap_or_default()
    }

    /// Remove the range that begins exactly at `page_index`.
    /// Returns `true` when a range was removed, `false` when none
    /// began there.
    pub fn remove_label(&mut self, page_index: i32) -> bool {
        self.ranges.remove(&page_index).is_some()
    }

    /// The start indices of every range, in ascending order.
    pub fn get_pages(&self) -> Vec<i32> {
        self.ranges.keys().copied().collect()
    }
}
