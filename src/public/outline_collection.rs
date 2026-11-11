//! `Aspose.Pdf.OutlineCollection` — the document's root bookmark
//! collection, per `capabilities/outline_collection.yaml`.
//!
//! Obtained from [`Document::outlines`](crate::Document::outlines).
//! The canonical type has no public constructor; items are created
//! against it via
//! [`OutlineItemCollection::new`](crate::OutlineItemCollection::new).
//! The .NET collection infrastructure (`CopyTo`, `GetEnumerator`,
//! `IsSynchronized`, `SyncRoot`, `IsReadOnly`) is dropped in favour
//! of Rust idioms (see `targets/rust/drops.yaml`).

use crate::public::outline_item_collection::OutlineItemCollection;
use crate::public::outlines::{self, Link, Node};
use std::fmt;
use std::rc::Rc;

/// The document's root collection of outline (bookmark) items.
#[derive(Clone)]
pub struct OutlineCollection(Link);

impl fmt::Debug for OutlineCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutlineCollection")
            .field("count", &self.count())
            .finish()
    }
}

impl OutlineCollection {
    /// Construct a fresh root collection. Crate-internal — the
    /// public entry point is [`Document::outlines`](crate::Document::outlines).
    pub(crate) fn new_root() -> Self {
        OutlineCollection(Node::new_root())
    }

    pub(crate) fn from_link(link: Link) -> Self {
        OutlineCollection(link)
    }

    /// Whether two handles refer to the same collection.
    pub fn ptr_eq(&self, other: &OutlineCollection) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Append `item` as a top-level outline.
    pub fn add(&self, item: &OutlineItemCollection) {
        outlines::attach(&self.0, item.link());
    }

    /// Remove every outline item.
    pub fn delete(&self) {
        outlines::clear_children(&self.0);
    }

    /// Remove the first top-level item whose title equals `name`.
    pub fn delete_by_name(&self, name: &str) {
        outlines::remove_child_named(&self.0, name);
    }

    /// Remove every outline item (alias of [`delete`](Self::delete)).
    pub fn clear(&self) {
        outlines::clear_children(&self.0);
    }

    /// Whether `item` is a top-level outline.
    pub fn contains(&self, item: &OutlineItemCollection) -> bool {
        outlines::contains_child(&self.0, item.link())
    }

    /// Remove `item`; returns whether it was present.
    pub fn remove(&self, item: &OutlineItemCollection) -> bool {
        outlines::remove_child(&self.0, item.link())
    }

    /// Remove the top-level item at a 1-based index (no-op when out
    /// of range).
    pub fn remove_at(&self, index: i32) {
        if index >= 1 {
            outlines::remove_child_at(&self.0, (index - 1) as usize);
        }
    }

    /// Whether the tree has been mutated since load — drives
    /// `Document::save`'s `/Outlines` save-through.
    pub(crate) fn is_dirty(&self) -> bool {
        outlines::is_dirty(&self.0)
    }

    /// Reset the dirty flag (after building a freshly loaded tree).
    pub(crate) fn clear_dirty(&self) {
        outlines::clear_dirty(&self.0);
    }

    /// The number of top-level items.
    pub fn count(&self) -> usize {
        outlines::child_count(&self.0)
    }

    /// The number of visible items when expanded per the open flags.
    pub fn visible_count(&self) -> i32 {
        outlines::visible_descendants(&self.0)
    }

    /// The first top-level item, if any.
    pub fn first(&self) -> Option<OutlineItemCollection> {
        outlines::child_at(&self.0, 0).map(OutlineItemCollection::from_link)
    }

    /// The last top-level item, if any.
    pub fn last(&self) -> Option<OutlineItemCollection> {
        let n = self.count();
        if n == 0 {
            None
        } else {
            outlines::child_at(&self.0, n - 1).map(OutlineItemCollection::from_link)
        }
    }

    /// The top-level item at a 1-based index.
    ///
    /// # Panics
    /// Panics when `index` is out of range (`1..=count`), matching
    /// the canonical indexer's bounds throw.
    pub fn get(&self, index: i32) -> OutlineItemCollection {
        assert!(index >= 1, "outline index is 1-based");
        outlines::child_at(&self.0, (index - 1) as usize)
            .map(OutlineItemCollection::from_link)
            .expect("outline index out of range")
    }
}
