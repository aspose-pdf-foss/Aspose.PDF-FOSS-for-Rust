//! `Aspose.Pdf.ImagePlacementCollection` — the placements produced by
//! an [`ImagePlacementAbsorber`](crate::ImagePlacementAbsorber), per
//! `capabilities/image_placement_collection.yaml`.
//!
//! A strict subset: the read surface — [`count`](ImagePlacementCollection::count)
//! and the 1-based indexer ([`get`](ImagePlacementCollection::get)).
//! The mutating `ICollection` surface and the .NET enumeration /
//! thread-sync infrastructure are expressed through Rust slices /
//! iterators instead (mirrors
//! [`TextFragmentCollection`](crate::TextFragmentCollection)).

use crate::public::image_placement::ImagePlacement;

/// A collection of [`ImagePlacement`]s. Indexed 1-based, matching the
/// canonical indexer.
#[derive(Debug, Default)]
pub struct ImagePlacementCollection {
    items: Vec<ImagePlacement>,
}

impl ImagePlacementCollection {
    /// The number of placements.
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// The placement at a 1-based index, or `None` if out of range.
    pub fn get(&self, index: usize) -> Option<&ImagePlacement> {
        index.checked_sub(1).and_then(|i| self.items.get(i))
    }

    /// Append a placement. Crate-internal — populated by an absorber.
    pub(crate) fn push(&mut self, placement: ImagePlacement) {
        self.items.push(placement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_collection_reads() {
        let c = ImagePlacementCollection::default();
        assert_eq!(c.count(), 0);
        assert!(c.get(1).is_none());
        assert!(c.get(0).is_none());
    }
}
