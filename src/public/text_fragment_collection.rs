//! `Aspose.Pdf.Text.TextFragmentCollection` — the fragments
//! produced by a text absorber, per
//! `capabilities/text_fragment_collection.yaml`.
//!
//! A strict subset: the read surface — `count` and the 1-based
//! indexer ([`get`](TextFragmentCollection::get)). The mutating
//! `ICollection` surface and the .NET enumeration / thread-sync
//! infrastructure are dropped in favour of Rust slices / iterators
//! (see `targets/rust/drops.yaml`). The collection is populated by
//! [`TextFragmentAbsorber`](crate::TextAbsorber)-style search; the
//! population path lands with that absorber.

use crate::public::text_fragment::TextFragment;

/// A collection of [`TextFragment`]s. Indexed 1-based, matching the
/// canonical indexer.
#[derive(Debug, Default)]
pub struct TextFragmentCollection {
    items: Vec<TextFragment>,
}

impl TextFragmentCollection {
    /// The number of fragments.
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// The fragment at a 1-based index, or `None` if out of range.
    pub fn get(&self, index: usize) -> Option<&TextFragment> {
        index.checked_sub(1).and_then(|i| self.items.get(i))
    }

    /// Append a located fragment. Crate-internal — populated by a
    /// text absorber.
    pub(crate) fn push(&mut self, fragment: TextFragment) {
        self.items.push(fragment);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_collection_reads() {
        let c = TextFragmentCollection::default();
        assert_eq!(c.count(), 0);
        assert!(c.get(1).is_none());
        assert!(c.get(0).is_none());
    }
}
