//! `Aspose.Pdf.EmbeddedFileCollection` — the collection of
//! embedded-file specifications attached to a document, per
//! `capabilities/embedded_file_collection.yaml`.
//!
//! Ships the keyed `add` / `add_with_key`, the deletion family
//! (`delete_by_key` / `delete_by_name` / `delete_all`),
//! `find_by_name`, `count`, `keys`, and the integer / name
//! indexers (`get` / `get_by_name`). The .NET collection
//! infrastructure (`CopyTo`, `GetEnumerator`, `IsSynchronized`,
//! `SyncRoot`) is dropped in favour of Rust's iterator idioms (see
//! `targets/rust/drops.yaml`).

use crate::public::error::Error;
use crate::public::file_specification::FileSpecification;

/// A collection of embedded-file specifications keyed by name.
///
/// Each entry has a key (the explicit key passed to
/// [`add_with_key`], otherwise the specification's source file
/// path) and a [`FileSpecification`] value. Integer indexing is
/// zero-based.
///
/// [`add_with_key`]: EmbeddedFileCollection::add_with_key
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EmbeddedFileCollection {
    entries: Vec<(String, FileSpecification)>,
    /// Set when mutated, so `Document::save` knows to (re)write the
    /// `/Names /EmbeddedFiles` name tree.
    dirty: bool,
}

impl EmbeddedFileCollection {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a specification, keyed by its source file path.
    pub fn add(&mut self, file: FileSpecification) {
        let key = file.file().to_string();
        self.entries.push((key, file));
        self.dirty = true;
    }

    /// Add a specification under an explicit key.
    pub fn add_with_key(&mut self, key: String, file: FileSpecification) {
        self.entries.push((key, file));
        self.dirty = true;
    }

    /// Remove the entry with the given key. No-op when absent.
    pub fn delete_by_key(&mut self, key: &str) {
        self.entries.retain(|(k, _)| k != key);
        self.dirty = true;
    }

    /// Remove the entry whose specification has the given name.
    /// No-op when absent.
    pub fn delete_by_name(&mut self, name: &str) {
        self.entries.retain(|(_, f)| f.name() != name);
        self.dirty = true;
    }

    /// Remove every entry.
    pub fn delete_all(&mut self) {
        self.entries.clear();
        self.dirty = true;
    }

    /// Whether the collection has been mutated since load.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Append an entry loaded from the PDF (does not mark the
    /// collection dirty, so a verbatim resave keeps the original).
    pub(crate) fn add_loaded(&mut self, key: String, file: FileSpecification) {
        self.entries.push((key, file));
    }

    /// `(key, &spec)` pairs, for save-through.
    pub(crate) fn entries_for_save(&self) -> Vec<(String, &FileSpecification)> {
        self.entries.iter().map(|(k, f)| (k.clone(), f)).collect()
    }

    /// Find a specification by its name, or `None` when absent.
    pub fn find_by_name(&self, name: &str) -> Option<&FileSpecification> {
        self.entries.iter().find(|(_, f)| f.name() == name).map(|(_, f)| f)
    }

    /// The number of entries.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// The entry keys, in insertion order.
    pub fn keys(&self) -> Vec<String> {
        self.entries.iter().map(|(k, _)| k.clone()).collect()
    }

    /// The specification at the (zero-based) index.
    ///
    /// Returns [`Error`] when the index is out of range — the Rust
    /// analogue of the canonical integer indexer's bounds throw.
    pub fn get(&self, index: usize) -> Result<&FileSpecification, Error> {
        self.entries
            .get(index)
            .map(|(_, f)| f)
            .ok_or_else(|| Error::CollectionItemNotFound(format!("index {index} out of range")))
    }

    /// The specification with the given name.
    ///
    /// Returns [`Error`] when no entry matches — the Rust analogue
    /// of the canonical name indexer's missing-key throw. Use
    /// [`find_by_name`] for a non-throwing lookup.
    ///
    /// [`find_by_name`]: EmbeddedFileCollection::find_by_name
    pub fn get_by_name(&self, name: &str) -> Result<&FileSpecification, Error> {
        self.find_by_name(name)
            .ok_or_else(|| Error::CollectionItemNotFound(format!("name {name:?}")))
    }
}
