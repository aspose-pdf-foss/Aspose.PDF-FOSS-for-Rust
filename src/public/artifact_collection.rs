//! `Aspose.Pdf.ArtifactCollection` — a page's artifacts, per
//! `capabilities/artifact_collection.yaml`.
//!
//! A 1-based collection of [`Artifact`]s. A shared, interior-mutable
//! handle (`Rc<RefCell<…>>`) so a page's artifacts can be mutated
//! through a shared `&Page` (via [`Page::artifacts`](crate::Page::artifacts))
//! and the change reaches the document's save path: on save, a text
//! artifact renders a positioned text run into the page content and an
//! image artifact embeds an image XObject in the page resources. The
//! object-identity `Delete(Artifact)` / `Update(Artifact)` overloads are
//! dropped; removal is by index.

use crate::public::artifact::Artifact;
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

/// An ordered, 1-based collection of [`Artifact`]s.
#[derive(Default, Clone)]
pub struct ArtifactCollection {
    items: Rc<RefCell<Vec<Artifact>>>,
    dirty: Rc<Cell<bool>>,
}

impl std::fmt::Debug for ArtifactCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArtifactCollection").field("count", &self.count()).finish()
    }
}

impl ArtifactCollection {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an artifact (or anything convertible to one, such as a
    /// [`WatermarkArtifact`](crate::WatermarkArtifact)).
    pub fn add(&self, artifact: impl Into<Artifact>) {
        self.items.borrow_mut().push(artifact.into());
        self.dirty.set(true);
    }

    /// Delete the artifact at a 1-based index.
    pub fn delete_at(&self, index: usize) {
        if let Some(i) = index.checked_sub(1) {
            let mut items = self.items.borrow_mut();
            if i < items.len() {
                items.remove(i);
                self.dirty.set(true);
            }
        }
    }

    /// The number of artifacts.
    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// The artifact at a 1-based index (a [`Ref`] guard), or `None` when
    /// out of range.
    pub fn get(&self, index: usize) -> Option<Ref<'_, Artifact>> {
        let i = index.checked_sub(1)?;
        let items = self.items.borrow();
        if i < items.len() {
            Some(Ref::map(items, |v| &v[i]))
        } else {
            None
        }
    }

    // ---- save-through support (crate-internal) ----

    /// Whether the collection was mutated since it was built.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// Reset the dirty flag (after rendering the artifacts on save).
    pub(crate) fn clear_dirty(&self) {
        self.dirty.set(false);
    }

    /// A snapshot of every artifact, for render-on-save.
    pub(crate) fn for_save(&self) -> Vec<Artifact> {
        self.items.borrow().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::watermark_artifact::WatermarkArtifact;

    #[test]
    fn add_index_and_delete() {
        let c = ArtifactCollection::new();
        let mut wm = WatermarkArtifact::new();
        wm.set_text("CONFIDENTIAL");
        c.add(wm);
        c.add(Artifact::new("Background", ""));
        assert_eq!(c.count(), 2);
        assert_eq!(c.get(1).unwrap().text(), "CONFIDENTIAL");
        assert!(c.get(0).is_none());
        c.delete_at(1);
        assert_eq!(c.count(), 1);
        // The watermark (with text) was removed; the background remains.
        assert_eq!(c.get(1).unwrap().text(), "");
    }

    #[test]
    fn shared_handle_sees_mutations() {
        let a = ArtifactCollection::new();
        let b = a.clone();
        a.add(WatermarkArtifact::new());
        assert_eq!(b.count(), 1);
    }
}
