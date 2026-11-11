//! `Aspose.Pdf.Facades.Bookmarks` — an ordered list of [`Bookmark`]s,
//! per `capabilities/bookmarks.yaml`.
//!
//! Canonically a `List<Bookmark>`; modelled as a 0-indexed Rust
//! collection with the list operations.

use crate::public::bookmark::Bookmark;

/// An ordered, 0-indexed list of [`Bookmark`]s.
#[derive(Debug, Clone, Default)]
pub struct Bookmarks {
    items: Vec<Bookmark>,
}

impl Bookmarks {
    /// Construct an empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a bookmark.
    pub fn add(&mut self, bookmark: Bookmark) {
        self.items.push(bookmark);
    }

    /// The number of bookmarks.
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The bookmark at a 0-based index.
    pub fn get(&self, index: usize) -> Option<&Bookmark> {
        self.items.get(index)
    }

    /// The bookmark at a 0-based index, mutably.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Bookmark> {
        self.items.get_mut(index)
    }

    /// Remove every bookmark.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Iterate the bookmarks.
    pub fn iter(&self) -> std::slice::Iter<'_, Bookmark> {
        self.items.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_is_a_list() {
        let mut list = Bookmarks::new();
        assert!(list.is_empty());
        let mut a = Bookmark::new();
        a.set_title("A");
        let mut b = Bookmark::new();
        b.set_title("B");
        list.add(a);
        list.add(b);
        assert_eq!(list.count(), 2);
        assert_eq!(list.get(0).unwrap().title(), "A");
        assert_eq!(list.get(1).unwrap().title(), "B");
    }
}
