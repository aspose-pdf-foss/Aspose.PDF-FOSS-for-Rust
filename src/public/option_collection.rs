//! `Aspose.Pdf.Forms.OptionCollection` — the collection of a choice
//! field's options, per `capabilities/option_collection.yaml`.
//!
//! The .NET collection infrastructure (`CopyTo` / `GetEnumerator` /
//! `IsSynchronized` / `IsReadOnly` / `SyncRoot`) is dropped in favour
//! of Rust idioms (see `targets/rust/drops.yaml`).

// The form `Option` shadows std `Option`; alias it locally so the
// std type stays available for the return values below.
use crate::public::option::Option as FormOption;

/// A choice field's options, indexed 0-based and addressable by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionCollection {
    items: Vec<FormOption>,
}

impl OptionCollection {
    /// Construct an empty option collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an option, assigning it the next index.
    pub fn add(&mut self, mut option: FormOption) {
        option.set_index(self.items.len() as i32);
        self.items.push(option);
    }

    /// Remove every option.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Whether `option` is present (by value equality).
    pub fn contains(&self, option: &FormOption) -> bool {
        self.items.contains(option)
    }

    /// Remove the first option equal to `option`; returns whether one
    /// was removed.
    pub fn remove(&mut self, option: &FormOption) -> bool {
        if let Some(pos) = self.items.iter().position(|o| o == option) {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }

    /// The number of options.
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// The option at a 0-based index.
    pub fn at(&self, index: usize) -> std::option::Option<&FormOption> {
        self.items.get(index)
    }

    /// The option with the given name.
    pub fn by_name(&self, name: &str) -> std::option::Option<&FormOption> {
        self.items.iter().find(|o| o.name() == name)
    }
}
