//! `Aspose.Pdf.Paragraphs` — an ordered collection of
//! [`BaseParagraph`]s, per `capabilities/paragraphs.yaml`.
//!
//! Holds the child paragraphs of a [`FloatingBox`](crate::FloatingBox)
//! or a [`Page`](crate::Page). A shared, interior-mutable handle
//! (`Rc<RefCell<…>>`) so a page's paragraphs can be mutated through a
//! shared `&Page` (via [`Page::paragraphs`](crate::Page::paragraphs))
//! and the change reaches the document's save path. The object-identity
//! `Remove` overload and the .NET enumeration / clone / range-by-value
//! helpers outside the shipped subset are dropped; mutation is by index.

use crate::public::base_paragraph::BaseParagraph;
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

/// An ordered, 0-indexed collection of paragraphs.
#[derive(Default, Clone)]
pub struct Paragraphs {
    items: Rc<RefCell<Vec<Box<dyn BaseParagraph>>>>,
    dirty: Rc<Cell<bool>>,
}

impl std::fmt::Debug for Paragraphs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Paragraphs").field("count", &self.count()).finish()
    }
}

impl Paragraphs {
    /// Construct an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a paragraph.
    pub fn add(&self, paragraph: Box<dyn BaseParagraph>) {
        self.items.borrow_mut().push(paragraph);
        self.dirty.set(true);
    }

    /// Insert a paragraph at a 0-based index (clamped to the end).
    pub fn insert(&self, index: usize, paragraph: Box<dyn BaseParagraph>) {
        let mut items = self.items.borrow_mut();
        let at = index.min(items.len());
        items.insert(at, paragraph);
        self.dirty.set(true);
    }

    /// Remove `count` paragraphs starting at a 0-based index
    /// (clamped to the collection bounds).
    pub fn remove_range(&self, index: usize, count: usize) {
        let mut items = self.items.borrow_mut();
        let len = items.len();
        if index >= len || count == 0 {
            return;
        }
        let end = index.saturating_add(count).min(len);
        items.drain(index..end);
        self.dirty.set(true);
    }

    /// Remove every paragraph.
    pub fn clear(&self) {
        self.items.borrow_mut().clear();
        self.dirty.set(true);
    }

    /// The number of paragraphs.
    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// The paragraph at a 0-based index (a [`Ref`] guard), or `None` if
    /// out of range.
    pub fn get(&self, index: usize) -> Option<Ref<'_, dyn BaseParagraph>> {
        let items = self.items.borrow();
        if index < items.len() {
            Some(Ref::map(items, |v| v[index].as_ref()))
        } else {
            None
        }
    }

    // -- save-through plumbing ---------------------------------------

    /// Whether the collection has been mutated since the page was
    /// built.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// Reset the dirty flag (after rendering the paragraphs on save).
    pub(crate) fn clear_dirty(&self) {
        self.dirty.set(false);
    }

    /// Render every paragraph (one entry per paragraph that renders
    /// anything), for the save path.
    pub(crate) fn render_all(
        &self,
        page_height: f64,
    ) -> Vec<crate::public::base_paragraph::sealed::RenderedParagraph> {
        self.items.borrow().iter().filter_map(|p| p.render_content(page_height)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::public::floating_box::FloatingBox;

    #[test]
    fn add_insert_remove_and_index() {
        let p = Paragraphs::new();
        assert_eq!(p.count(), 0);
        p.add(Box::new(FloatingBox::with_dimensions(10.0, 20.0)));
        p.add(Box::new(FloatingBox::with_dimensions(30.0, 40.0)));
        assert_eq!(p.count(), 2);
        p.insert(1, Box::new(FloatingBox::with_dimensions(50.0, 60.0)));
        assert_eq!(p.count(), 3);
        assert!(p.get(0).is_some());
        assert!(p.get(3).is_none());
        p.remove_range(0, 2);
        assert_eq!(p.count(), 1);
        p.clear();
        assert_eq!(p.count(), 0);
    }
}
