//! `Aspose.Pdf.NamedDestinationCollection` — the document's
//! collection of named navigation targets, per
//! `capabilities/named_destination_collection.yaml`.
//!
//! Maps each name to an [`IAppointment`] destination. `add`
//! replaces an existing same-named entry; `get` returns the stored
//! appointment or `None` (the canonical name indexer is nullable).
//! Insertion order is preserved for `names`.

use crate::public::i_appointment::IAppointment;
use std::fmt;

/// A collection of named navigation targets keyed by name.
#[derive(Default)]
pub struct NamedDestinationCollection {
    entries: Vec<(String, Box<dyn IAppointment>)>,
    /// Set when mutated, so `Document::save` knows to (re)write the
    /// `/Names /Dests` name tree.
    dirty: bool,
}

impl NamedDestinationCollection {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add (or replace) the destination stored under `name`.
    pub fn add(&mut self, name: String, appointment: Box<dyn IAppointment>) {
        if let Some(slot) = self.entries.iter_mut().find(|(n, _)| *n == name) {
            slot.1 = appointment;
        } else {
            self.entries.push((name, appointment));
        }
        self.dirty = true;
    }

    /// Remove the destination stored under `name`. No-op when
    /// absent.
    pub fn remove(&mut self, name: &str) {
        self.entries.retain(|(n, _)| n != name);
        self.dirty = true;
    }

    /// Whether the collection has been mutated since load.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// `(name, destination-string)` pairs, for save-through.
    pub(crate) fn entries_for_save(&self) -> Vec<(String, String)> {
        self.entries.iter().map(|(n, d)| (n.clone(), d.to_string())).collect()
    }

    /// Append a destination loaded from the PDF (does not mark the
    /// collection dirty, so a verbatim resave keeps the original).
    pub(crate) fn add_loaded(&mut self, name: String, appointment: Box<dyn IAppointment>) {
        self.entries.push((name, appointment));
    }

    /// The number of entries.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// The destination names, in insertion order.
    pub fn names(&self) -> Vec<String> {
        self.entries.iter().map(|(n, _)| n.clone()).collect()
    }

    /// The destination stored under `name`, or `None` when absent.
    pub fn get(&self, name: &str) -> Option<&dyn IAppointment> {
        self.entries
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, d)| d.as_ref())
    }
}

impl fmt::Debug for NamedDestinationCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NamedDestinationCollection")
            .field("names", &self.names())
            .finish()
    }
}
